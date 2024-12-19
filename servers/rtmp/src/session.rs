use core::time;
use std::{
    backtrace::Backtrace,
    borrow::BorrowMut,
    cmp::min,
    collections::HashMap,
    io::{self, Cursor},
    time::{Duration, SystemTime},
};

use ::stream_center::{
    events::StreamCenterEvent,
    frame_info::{AggregateMeta, AudioMeta, FrameData, VideoMeta},
    gop::GopQueue,
    stream_center::StreamCenter,
    stream_source::{StreamIdentifier, StreamType},
};
use rtmp_formats::{
    chunk::{
        self, ChunkMessage, ChunkMessageCommonHeader, RtmpChunkMessageBody,
        errors::ChunkMessageError,
    },
    commands::{
        CallCommandRequest, ConnectCommandRequest, CreateStreamCommandRequest, DeleteStreamCommand,
        PauseCommand, Play2Command, PlayCommand, PublishCommand, ReceiveAudioCommand,
        ReceiveVideoCommand, RtmpC2SCommands, SeekCommand, consts::RESPONSE_STREAM_ID,
    },
    handshake,
    message::RtmpUserMessageBody,
    protocol_control::{
        AbortMessage, Acknowledgement, ProtocolControlMessage, SetChunkSize,
        SetPeerBandWidthLimitType, SetPeerBandwidth, WindowAckSize,
    },
    user_control::UserControlEvent,
};
use stream_center::stream_center;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufWriter},
    net::TcpStream,
    runtime::Handle,
    sync::{
        broadcast::{self, error::RecvError},
        mpsc, oneshot,
    },
    time::timeout,
};
use tokio_util::{
    bytes::{Buf, BytesMut},
    either::Either,
};
use tracing::{Instrument, instrument};
use uuid::Uuid;

use crate::errors::RtmpServerError;

use super::{
    config::RtmpSessionConfig,
    consts::{response_code, response_level},
    errors::RtmpServerResult,
};

#[derive(Debug)]
enum SessionRuntime {
    Play {
        stream_data_consumer: mpsc::Receiver<FrameData>,
        stream_type: StreamType,
        play_id: Uuid,
        receive_audio: bool,
        receive_video: bool,
        buffer_length: Option<u32>,
    },
    Publish {
        stream_data_producer: mpsc::Sender<FrameData>,
        no_data_since: Option<time::Duration>,
    },
    PublishStop {
        stop_time: SystemTime,
    },
    PlayStop,
    Unknown,
}

#[derive(Debug, Default)]
struct StreamProperties {
    stream_name: String,
    app: String,
    tc_url: String,
    swf_url: String,
    page_url: String,
    amf_version: amf::Version,

    stream_type: StreamType,

    stream_context: HashMap<String, serde_json::Value>,
}

#[derive(Debug)]
pub struct RtmpSession {
    read_buffer: BytesMut,
    stream: BufWriter<TcpStream>,
    chunk_reader: chunk::reader::Reader,
    chunk_writer: chunk::writer::Writer,

    runtime_handle: SessionRuntime,

    stream_properties: StreamProperties,

    ack_window_size_read: Option<u32>,
    ack_window_size_write: Option<SetPeerBandwidth>,

    acknowledged_sequence_number: Option<u32>,
    total_wrote_bytes: usize,

    config: RtmpSessionConfig,

    stream_center_event_sender: mpsc::UnboundedSender<StreamCenterEvent>,
}

impl RtmpSession {
    pub fn new(
        io: TcpStream,
        stream_center_event_sender: mpsc::UnboundedSender<StreamCenterEvent>,
        config: RtmpSessionConfig,
    ) -> Self {
        Self {
            read_buffer: BytesMut::with_capacity(4096),
            stream: BufWriter::new(io),
            chunk_reader: chunk::reader::Reader::new(),
            chunk_writer: chunk::writer::Writer::new(),

            stream_properties: StreamProperties::default(),
            runtime_handle: SessionRuntime::Unknown,

            ack_window_size_read: None,
            ack_window_size_write: None,
            acknowledged_sequence_number: None,
            total_wrote_bytes: 0,

            config,

            stream_center_event_sender,
        }
    }

    async fn read_chunk(&mut self) -> RtmpServerResult<Option<ChunkMessage>> {
        loop {
            let mut buf = Cursor::new(&self.read_buffer[..]);
            match self.chunk_reader.read(&mut buf, true) {
                Ok(Some(chunk_message)) => {
                    self.read_buffer.advance(buf.position() as usize);
                    return Ok(Some(chunk_message));
                }
                Ok(None) => {}
                Err(ChunkMessageError::IncompleteChunk) => {
                    self.read_buffer.advance(buf.position() as usize);
                    return Ok(None);
                }
                Err(err) => return Err(err.into()),
            }

            match self.ack_window_size_read {
                None => {}
                Some(size) => {
                    if self.chunk_reader.get_bytes_read() >= size {
                        self.ack_window_size(size as u32).await?;
                    }
                }
            }

            match tokio::time::timeout(
                time::Duration::from_millis(self.config.read_timeout_ms),
                self.stream.read_buf(&mut self.read_buffer),
            )
            .await
            {
                Ok(Ok(len)) => {
                    if len == 0 {
                        if self.read_buffer.is_empty() {
                            return Ok(None);
                        } else {
                            return Err(RtmpServerError::Io(io::Error::new(
                                io::ErrorKind::ConnectionReset,
                                "connect reset by peer",
                            )));
                        }
                    }
                }
                Ok(Err(err)) => return Err(err.into()),
                Err(err) => {
                    return Err(RtmpServerError::Io(io::Error::new(
                        io::ErrorKind::TimedOut,
                        format!("read chunk data timeout: {}", err),
                    )));
                }
            }
        }
    }

    async fn flush_chunk(&mut self) -> RtmpServerResult<()> {
        let flushable = match &self.ack_window_size_write {
            None => true,
            Some(limit) => {
                let unacknowledged_bytes = self.total_wrote_bytes
                    - self.acknowledged_sequence_number.unwrap_or(0) as usize;
                unacknowledged_bytes < limit.size as usize
            }
        };
        if !flushable {
            return Ok(());
        }
        self.chunk_writer.write_to(&mut self.stream).await?;

        timeout(
            Duration::from_millis(self.config.write_timeout_ms as u64),
            async move {
                self.stream.flush().await?;
                self.total_wrote_bytes = self.chunk_writer.get_bytes_written();
                Ok::<(), RtmpServerError>(())
            },
        )
        .await
        .map_err(|err| {
            return RtmpServerError::Io(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("write chunk timeout, {}", err),
            ));
        })??;
        Ok(())
    }

    pub async fn run(&mut self) -> RtmpServerResult<()> {
        handshake::server::HandshakeServer::new(&mut self.stream)
            .handshake(false)
            .await?;
        self.chunk_writer.write_set_chunk_size(4096)?;

        loop {
            let play_id = match &self.runtime_handle {
                SessionRuntime::Play {
                    stream_data_consumer: _,
                    stream_type: _,
                    play_id,
                    receive_audio: _,
                    receive_video: _,
                    buffer_length: _,
                } => Some(play_id.clone()),
                _ => None,
            };
            if let Some(play_id) = play_id {
                let res = self.playing().await;
                if res.is_err() {
                    let unsubscribe_result = self
                        .unsubscribe_from_stream_center(
                            play_id.clone(),
                            &self.stream_properties.stream_name.clone(),
                            &self.stream_properties.app.clone(),
                        )
                        .await;
                }

                self.runtime_handle = SessionRuntime::PlayStop;
            }

            match self.read_chunk().await {
                Ok(maybe_chunk) => match maybe_chunk {
                    Some(message) => {
                        tracing::trace!("got message: {:?}", message);
                        self.process_message(message).await?;
                    }
                    None => match &self.runtime_handle {
                        SessionRuntime::PublishStop { stop_time } => {
                            let current_time = SystemTime::now();
                            if current_time
                                .duration_since(*stop_time)
                                .expect("stop time must be before")
                                .as_secs()
                                > 10
                            {
                                // 10 seconds after publish stop, and no data received, we close this session
                                tracing::info!("publish session timeout, closing");
                                return Ok(());
                            }
                        }
                        _ => {}
                    },
                },
                Err(err) => match err {
                    RtmpServerError::ChunkMessageReadFailed(
                        ChunkMessageError::UnknownMessageType { type_id, backtrace },
                    ) => {
                        tracing::error!(
                            "got unknown message: type_id: {}, backtrace: {}",
                            type_id,
                            backtrace
                        );
                    }
                    RtmpServerError::Io(io_err) => {
                        if io_err.kind() == io::ErrorKind::ConnectionReset {
                            tracing::info!("connect reset by peer");
                            return Ok(());
                        }
                        tracing::error!("io error: {:?}", io_err);
                    }
                    err => {
                        tracing::error!("{:?}", err);
                        panic!();
                    }
                },
            }
        }
    }

    async fn playing(&mut self) -> RtmpServerResult<()> {
        let mut messages = Vec::with_capacity(1280);
        loop {
            messages.clear();
            match &mut self.runtime_handle {
                SessionRuntime::Play {
                    stream_data_consumer,
                    play_id: _,
                    stream_type: _,
                    receive_audio: _,
                    receive_video: _,
                    buffer_length: _,
                } => match stream_data_consumer.recv_many(&mut messages, 1280).await {
                    0 => {
                        tracing::error!("channel closed while trying to play");
                        return Err(RtmpServerError::StreamIsGone);
                    }
                    len => {
                        tracing::info!("got {} messages from stream source", len);
                        for message in &messages {
                            match message {
                                FrameData::Video { meta: _, data } => {
                                    self.chunk_writer.write_video(data.clone())?;
                                }
                                FrameData::Audio { meta: _, data } => {
                                    self.chunk_writer.write_audio(data.clone())?
                                }
                                FrameData::Aggregate { meta: _, data } => {
                                    tracing::info!("got an aggregate, ignore for now");
                                }
                                FrameData::Meta { timestamp: _, data } => {
                                    self.chunk_writer.write_meta(data.clone())?;
                                }
                            }
                        }
                    }
                },
                _ => {}
            }
            self.flush_chunk().await?;
        }
    }

    async fn process_message(&mut self, message: ChunkMessage) -> RtmpServerResult<()> {
        let header = message.header;
        let body = message.chunk_message_body;
        match body {
            RtmpChunkMessageBody::ProtocolControl(request) => {
                self.process_protocol_control_message(request).await?
            }
            RtmpChunkMessageBody::UserControl(control) => {
                self.process_user_control_event(control).await?
            }
            RtmpChunkMessageBody::RtmpUserMessage(message) => {
                self.process_user_message(message, header).await?
            }
        }
        Ok(())
    }

    async fn process_user_message(
        &mut self,
        message: RtmpUserMessageBody,
        header: ChunkMessageCommonHeader,
    ) -> RtmpServerResult<()> {
        let _ = match message {
            RtmpUserMessageBody::C2SCommand(command) => {
                self.process_user_command(command, header).await?
            }
            RtmpUserMessageBody::MetaData(meta) => self.process_meta(meta).await?,
            RtmpUserMessageBody::Aggregate { payload } => self.process_aggregate(payload).await?,
            RtmpUserMessageBody::Audio { payload } => self.process_audio(payload).await?,
            RtmpUserMessageBody::Video { payload } => self.process_video(payload).await?,
            RtmpUserMessageBody::S2Command(command) => {
                tracing::error!("got unexpected s2c command: {:?}", command);
            }
            RtmpUserMessageBody::SharedObject() => {
                tracing::warn!("ignore shared object command");
            }
        };
        Ok(())
    }

    async fn process_audio(&mut self, audio: BytesMut) -> RtmpServerResult<()> {
        let _ = match &mut self.runtime_handle {
            SessionRuntime::Publish {
                stream_data_producer,
                no_data_since,
            } => {
                *no_data_since = None;
                stream_data_producer
                    .send(FrameData::Audio {
                        meta: AudioMeta::default(),
                        data: audio,
                    })
                    .await
                    .map_err(|err| {
                        tracing::error!("send audio data to stream center failed: {:?}", err);
                        RtmpServerError::ChannelSendFailed {
                            backtrace: Backtrace::capture(),
                        }
                    })?;
                tracing::info!("send audio");
            }
            _ => {
                tracing::error!(
                    "got audio data while stream not published, length: {}",
                    audio.len()
                );
            }
        };
        Ok(())
    }

    async fn process_video(&mut self, video: BytesMut) -> RtmpServerResult<()> {
        let _ = match &mut self.runtime_handle {
            SessionRuntime::Publish {
                stream_data_producer,
                no_data_since,
            } => {
                *no_data_since = None;

                stream_data_producer
                    .send(FrameData::Video {
                        meta: VideoMeta::default(),
                        data: video,
                    })
                    .await
                    .map_err(|err| {
                        tracing::error!("send video to stream center failed: {:?}", err);
                        RtmpServerError::ChannelSendFailed {
                            backtrace: Backtrace::capture(),
                        }
                    })?;
            }
            _ => {
                tracing::error!(
                    "got video data while stream not published, length: {}",
                    video.len()
                );
            }
        };
        Ok(())
    }

    async fn process_meta(&mut self, meta: amf::Value) -> RtmpServerResult<()> {
        let _ = match &mut self.runtime_handle {
            SessionRuntime::Publish {
                stream_data_producer,
                no_data_since,
            } => {
                *no_data_since = None;

                stream_data_producer
                    .send(FrameData::Meta {
                        timestamp: 0,
                        data: meta,
                    })
                    .await
                    .map_err(|err| {
                        tracing::error!("send meta to stream center failed: {:?}", err);
                        RtmpServerError::ChannelSendFailed {
                            backtrace: Backtrace::capture(),
                        }
                    })?;
            }
            _ => {
                tracing::error!(
                    "got meta data while stream not published, value: {:?}",
                    meta
                );
            }
        };
        Ok(())
    }

    async fn process_aggregate(&mut self, aggregate: BytesMut) -> RtmpServerResult<()> {
        let _ = match &mut self.runtime_handle {
            SessionRuntime::Publish {
                stream_data_producer,
                no_data_since,
            } => {
                *no_data_since = None;

                stream_data_producer
                    .send(FrameData::Aggregate {
                        meta: AggregateMeta::default(),
                        data: aggregate,
                    })
                    .await
                    .map_err(|err| RtmpServerError::ChannelSendFailed {
                        backtrace: Backtrace::capture(),
                    })?;
            }
            _ => {
                tracing::error!(
                    "got aggregate data while stream not published, length: {}",
                    aggregate.len()
                );
            }
        };
        Ok(())
    }

    async fn process_user_command(
        &mut self,
        command: RtmpC2SCommands,
        header: ChunkMessageCommonHeader,
    ) -> RtmpServerResult<()> {
        match command {
            RtmpC2SCommands::Connect(request) => {
                self.process_connect_command(request).await?;
            }
            RtmpC2SCommands::Call(request) => self.process_call_request(request).await?,
            RtmpC2SCommands::CreateStream(request) => {
                self.process_create_stream_command(request).await?
            }
            RtmpC2SCommands::DeleteStream(request) => {
                self.process_delete_stream_command(request).await?
            }
            RtmpC2SCommands::Pause(request) => self.process_pause_request(request)?,
            RtmpC2SCommands::Play(request) => self.process_play_request(request, header).await?,
            RtmpC2SCommands::Play2(request) => self.process_play2_request(request)?,
            RtmpC2SCommands::Publish(request) => {
                self.process_publish_command(request).await?;
            }
            RtmpC2SCommands::ReceiveAudio(request) => {
                self.process_receive_audio_request(request)?
            }
            RtmpC2SCommands::ReceiveVideo(request) => {
                self.process_receive_video_request(request)?
            }
            RtmpC2SCommands::Seek(request) => self.process_seek_request(request)?,
        };
        Ok(())
    }

    async fn process_connect_command(
        &mut self,
        request: ConnectCommandRequest,
    ) -> RtmpServerResult<()> {
        self.chunk_writer.write_window_ack_size_message(4096)?;
        self.chunk_writer
            .write_set_peer_bandwidth(4096, SetPeerBandWidthLimitType::Dynamic)?;
        self.flush_chunk().await?;

        self.chunk_writer.write_connect_response(
            true,
            request.transaction_id.into(),
            super::consts::FMSVER,
            super::consts::FMS_CAPABILITIES,
            super::consts::response_code::NET_CONNECTION_CONNECT_SUCCESS,
            super::consts::response_level::STATUS,
            "Connection Succeeded.",
            amf::Version::Amf0,
        )?;
        self.flush_chunk().await?;

        self.stream_properties.app = request.command_object.app;
        self.stream_properties.tc_url = request.command_object.tc_url;
        self.stream_properties.swf_url = request.command_object.swf_url;
        self.stream_properties.page_url = request.command_object.page_url;
        self.stream_properties.amf_version = request.command_object.object_encoding;

        Ok(())
    }

    async fn process_create_stream_command(
        &mut self,
        request: CreateStreamCommandRequest,
    ) -> RtmpServerResult<()> {
        self.chunk_writer.write_create_stream_response(
            true,
            request.transaction_id,
            None,
            RESPONSE_STREAM_ID.into(),
        )?;
        self.flush_chunk().await?;
        Ok(())
    }

    async fn process_publish_command(&mut self, request: PublishCommand) -> RtmpServerResult<()> {
        let stream_type: StreamType = request.publishing_type.try_into()?;
        self.publish_to_stream_center(&request.publishing_name, stream_type)
            .await?;

        self.chunk_writer.write_on_status_response(
            response_level::STATUS,
            response_code::NET_STREAM_PUBLISH_START_SUCCESS,
            "publish start",
            self.stream_properties.amf_version,
        )?;
        self.flush_chunk().await?;

        tracing::info!("process publish command success");
        Ok(())
    }

    async fn unpublish_from_stream_center(
        &mut self,
        stream_name: &str,
        app: &str,
    ) -> RtmpServerResult<()> {
        let (tx, rx) = oneshot::channel();
        self.stream_center_event_sender
            .send(StreamCenterEvent::Unpublish {
                stream_id: StreamIdentifier {
                    stream_name: stream_name.to_string(),
                    app: app.to_string(),
                },
                result_sender: tx,
            })
            .map_err(|err| {
                tracing::error!(
                    "send unpublish event to stream center failed, {:?}. stream_name: {}, app: {}",
                    err,
                    stream_name,
                    app
                );
                return RtmpServerError::ChannelSendFailed {
                    backtrace: Backtrace::capture(),
                };
            })?;

        match rx.await {
            Err(err) => {
                tracing::error!("channel closed while trying to receive unpublish result");
                return Err(RtmpServerError::ChannelSendFailed {
                    backtrace: Backtrace::capture(),
                });
            }
            Ok(Err(err)) => {
                tracing::error!(
                    "stream unpublish from stream center failed, {:?}. stream_name: {}, app: {}",
                    err,
                    stream_name,
                    app
                );
            }
            Ok(Ok(())) => {
                tracing::info!(
                    "unpublish from stream center success, stream_name: {}, app: {}",
                    stream_name,
                    app
                );
            }
        }

        Ok(())
    }

    async fn process_delete_stream_command(
        &mut self,
        request: DeleteStreamCommand,
    ) -> RtmpServerResult<()> {
        let _ = self
            .unpublish_from_stream_center(
                &self.stream_properties.stream_name.clone(),
                &self.stream_properties.app.clone(),
            )
            .await;

        self.runtime_handle = SessionRuntime::PublishStop {
            stop_time: SystemTime::now(),
        };

        self.chunk_writer.write_on_status_response(
            response_level::STATUS,
            response_code::NET_STREAM_DELETE_SUCCESS,
            "delete stream success",
            self.stream_properties.amf_version,
        )?;

        self.flush_chunk().await?;

        tracing::info!("process delete stream command success");
        Ok(())
    }

    async fn process_protocol_control_message(
        &mut self,
        request: ProtocolControlMessage,
    ) -> RtmpServerResult<()> {
        match request {
            ProtocolControlMessage::SetChunkSize(request) => {
                self.process_set_chunk_size_request(request);
            }
            ProtocolControlMessage::Abort(request) => self.process_abort_chunk_request(request),
            ProtocolControlMessage::Ack(request) => {
                self.process_acknowledge_request(request);
            }
            ProtocolControlMessage::SetPeerBandwidth(request) => {
                self.process_set_peer_bandwidth_request(request).await?;
            }
            ProtocolControlMessage::WindowAckSize(request) => {
                self.process_window_ack_size_request(request);
            }
        }
        Ok(())
    }

    fn process_set_chunk_size_request(&mut self, request: SetChunkSize) {
        let chunk_size = request.chunk_size;
        let old_size = self.chunk_reader.set_chunk_size(chunk_size as usize);
        tracing::trace!(
            "update read chunk size, from {} to {}",
            old_size,
            chunk_size
        );
    }

    fn process_abort_chunk_request(&mut self, request: AbortMessage) {
        tracing::info!("got abort request: {:?}", request);
        self.chunk_reader
            .abort_chunk_message(request.chunk_stream_id);
    }

    fn process_window_ack_size_request(&mut self, request: WindowAckSize) {
        tracing::info!("got window_ack_size request: {:?}", request);
        self.ack_window_size_read = Some(request.size);
    }

    async fn ack_window_size(&mut self, size: u32) -> RtmpServerResult<()> {
        tracing::info!("do ack: {}", size);
        self.chunk_writer.write_acknowledgement_message(size)?;
        self.flush_chunk().await?;
        Ok(())
    }

    async fn process_set_peer_bandwidth_request(
        &mut self,
        request: SetPeerBandwidth,
    ) -> RtmpServerResult<()> {
        tracing::info!("got set_peer_bandwidth request: {:?}", request);
        let mut window_ack_size = None;
        match &mut self.ack_window_size_write {
            None => self.ack_window_size_write = Some(request),
            Some(limit) => match request.limit_type {
                SetPeerBandWidthLimitType::Hard => {
                    if limit.size != request.size {
                        window_ack_size = Some(request.size);
                    }
                    *limit = request
                }
                SetPeerBandWidthLimitType::Soft => {
                    if request.size != limit.size {
                        window_ack_size = Some(request.size);
                    }
                    limit.size = min(limit.size, request.size)
                }
                SetPeerBandWidthLimitType::Dynamic => {
                    if limit.limit_type == SetPeerBandWidthLimitType::Hard {
                        if limit.size != request.size {
                            window_ack_size = Some(request.size);
                        }
                        limit.size = request.size;
                    } else {
                        tracing::trace!(
                            "ignore set_peer_bandwidth command as documented by the spec, req: {:?}",
                            request
                        );
                    }
                }
            },
        }

        if window_ack_size.is_some() {
            self.chunk_writer
                .write_window_ack_size_message(window_ack_size.expect("this cannot be none"))?;
            self.flush_chunk().await?;
        }
        Ok(())
    }

    fn process_acknowledge_request(&mut self, request: Acknowledgement) {
        tracing::info!("got acknowledge request: {:?}", request);
        self.acknowledged_sequence_number = Some(request.sequence_number);
    }

    async fn publish_to_stream_center(
        &mut self,
        stream_name: &str,
        stream_type: StreamType,
    ) -> RtmpServerResult<()> {
        match self.runtime_handle {
            SessionRuntime::Publish {
                stream_data_producer: _,
                no_data_since: _,
            } => return Ok(()),
            _ => {}
        };

        if stream_name.is_empty() {
            return Err(RtmpServerError::InvalidStreamParam);
        }

        self.stream_properties.stream_name = stream_name.to_string();
        self.stream_properties.stream_type = stream_type;

        let (tx, rx) = oneshot::channel();
        self.stream_center_event_sender
            .send(StreamCenterEvent::Publish {
                stream_type: self.stream_properties.stream_type,
                stream_id: StreamIdentifier {
                    stream_name: self.stream_properties.stream_name.clone(),
                    app: self.stream_properties.app.clone(),
                },
                context: self.stream_properties.stream_context.clone(),
                result_sender: tx,
            })
            .map_err(|err| {
                tracing::error!(
                    "send publish event to stream center failed, {:?}. stream_name: {}, app: {}",
                    err,
                    stream_name,
                    self.stream_properties.app.clone(),
                );
                return RtmpServerError::ChannelSendFailed {
                    backtrace: Backtrace::capture(),
                };
            })?;

        match rx.await {
            Err(err) => {
                tracing::error!("channel closed while trying to receive publish result");
                return Err(RtmpServerError::ChannelSendFailed {
                    backtrace: Backtrace::capture(),
                });
            }
            Ok(Err(err)) => {
                tracing::error!(
                    "publish to stream center failed, {:?}. stream_name: {},  app: {}, stream_type: {}",
                    err,
                    self.stream_properties.stream_name,
                    self.stream_properties.app,
                    self.stream_properties.stream_type,
                );
                return Err(err.into());
            }
            Ok(Ok(sender)) => {
                self.runtime_handle = SessionRuntime::Publish {
                    stream_data_producer: sender,
                    no_data_since: None,
                };
                tracing::info!(
                    "publish to stream center success, stream_name: {}, app: {}",
                    self.stream_properties.stream_name,
                    self.stream_properties.app
                );
            }
        }

        Ok(())
    }

    async fn process_call_request(&mut self, request: CallCommandRequest) -> RtmpServerResult<()> {
        let command_name = &request.procedure_name;
        match command_name.as_str() {
            "releaseStream" | "FCPublish" | "FCUnpublish" => {
                //NOTE - publish start command, same as createStream or publish
                let stream_name = match &request.optional_arguments {
                    None => None,
                    Some(v) => match v {
                        Either::Left(amf_any) => {
                            amf_any.try_as_str().map_or_else(|| None, |v| Some(v))
                        }
                        Either::Right(_map) => None,
                    },
                };

                match stream_name {
                    None => {
                        tracing::warn!(
                            "ignore call request as no stream name provided. {:?}",
                            request
                        );
                    }
                    Some(stream_name) => {
                        // ignore the result
                        let res = if command_name == "FCUnpublish" {
                            let res = self
                                .unpublish_from_stream_center(
                                    &self.stream_properties.stream_name.clone(),
                                    &self.stream_properties.app.clone(),
                                )
                                .await;
                            self.runtime_handle = SessionRuntime::PublishStop {
                                stop_time: SystemTime::now(),
                            };
                            res.map_err(|err| err.into())
                        } else {
                            self.publish_to_stream_center(stream_name, StreamType::default())
                                .await
                        };

                        self.chunk_writer.write_call_response(
                            res.is_ok(),
                            request.transaction_id,
                            None,
                            None,
                        )?;
                        self.flush_chunk().await?;
                    }
                }
            }
            _ => {
                tracing::warn!("got a call request, ignore. request: {:?}", request);
            }
        }
        Ok(())
    }

    fn process_pause_request(&mut self, request: PauseCommand) -> RtmpServerResult<()> {
        tracing::warn!("got a pause request, ignore. request: {:?}", request);
        Ok(())
    }

    async fn subscribe_from_stream_center(
        &mut self,
        stream_name: &str,
        app: &str,
    ) -> RtmpServerResult<(Uuid, StreamType, mpsc::Receiver<FrameData>)> {
        let (tx, rx) = oneshot::channel();
        self.stream_center_event_sender
            .send(StreamCenterEvent::Subscribe {
                stream_id: StreamIdentifier {
                    stream_name: stream_name.to_string(),
                    app: app.to_string(),
                },
                result_sender: tx,
            })
            .map_err(|err| {
                tracing::error!(
                    "send subscribe event to stream center failed, {:?}. stream_name: {}, app: {}",
                    err,
                    stream_name,
                    app
                );
                return RtmpServerError::ChannelSendFailed {
                    backtrace: Backtrace::capture(),
                };
            })?;

        match rx.await {
            Err(err) => {
                tracing::error!("channel closed while trying receive subscribe result");
                return Err(RtmpServerError::ChannelSendFailed {
                    backtrace: Backtrace::capture(),
                });
            }
            Ok(Err(err)) => {
                tracing::error!(
                    "subscribe from stream center failed, {:?}, stream_name: {}, app: {}",
                    err,
                    stream_name,
                    app
                );
                return Err(err.into());
            }
            Ok(Ok((uuid, stream_type, receiver))) => {
                tracing::info!(
                    "subscribe from stream center success, stream_name: {}, app: {}",
                    stream_name,
                    app
                );
                Ok((uuid, stream_type, receiver))
            }
        }
    }

    async fn unsubscribe_from_stream_center(
        &mut self,
        uuid: Uuid,
        stream_name: &str,
        app: &str,
    ) -> RtmpServerResult<()> {
        let (tx, rx) = oneshot::channel();
        self.stream_center_event_sender
            .send(StreamCenterEvent::Unsubscribe {
                stream_id: StreamIdentifier {
                    stream_name: stream_name.to_string(),
                    app: app.to_string(),
                },
                uuid,
                result_sender: tx,
            })
            .map_err(|err| {
                tracing::error!(
                    "send unsubscribe event to stream center failed, {:?}. stream_name: {}, app: {}, uuid: {}",
                    err,
                    stream_name,
                    app,
                    uuid,
                );
                return RtmpServerError::ChannelSendFailed {
                    backtrace: Backtrace::capture(),
                };
            })?;

        match rx.await {
            Err(err) => {
                tracing::error!("channel closed while trying to receive unsubscribe result");
                return Err(RtmpServerError::ChannelSendFailed {
                    backtrace: Backtrace::capture(),
                });
            }
            Ok(Err(err)) => {
                tracing::error!(
                    "unsubscribe from stream center failed, uuid: {}, stream_name: {}, app: {}",
                    uuid,
                    stream_name,
                    app
                );
                return Err(err.into());
            }
            Ok(Ok(())) => {
                tracing::info!(
                    "unsubscribe from stream center success, uuid: {}, stream_name: {}, app: {}",
                    uuid,
                    stream_name,
                    app
                );
            }
        }
        Ok(())
    }

    async fn process_play_request(
        &mut self,
        request: PlayCommand,
        header: ChunkMessageCommonHeader,
    ) -> RtmpServerResult<()> {
        tracing::info!("got play request: {:?}", request);
        let stream_name = request.stream_name;
        let start = request.start; // this might by useful
        let duration = request.duration; // this might by useful
        let reset = request.reset; // this should be ignored

        self.stream_properties.stream_name = stream_name.clone();
        let subscribe_result = self
            .subscribe_from_stream_center(stream_name.as_str(), &self.stream_properties.app.clone())
            .await;

        self.chunk_writer
            .write_set_chunk_size(self.config.chunk_size)?;
        self.flush_chunk().await?;
        if self.stream_properties.stream_type == StreamType::Record {
            self.chunk_writer
                .write_stream_ids_recorded(header.message_stream_id)?; // I bet the stream_id is useless
            self.flush_chunk().await?;
        }

        self.chunk_writer
            .write_stream_begin(header.message_stream_id)?;
        self.flush_chunk().await?;
        match subscribe_result {
            Err(err) => {
                tracing::error!("subscribe stream failed: {:?}", err);
                self.chunk_writer.write_on_status_response(
                    response_level::ERROR,
                    response_code::NET_STREAM_PLAY_NOT_FOUND,
                    "stream not found",
                    self.stream_properties.amf_version,
                )?;
            }
            Ok((uuid, stream_type, receiver)) => {
                self.runtime_handle = SessionRuntime::Play {
                    stream_data_consumer: receiver,
                    stream_type,
                    receive_audio: true,
                    receive_video: true,
                    buffer_length: None,
                    play_id: uuid,
                };
                if reset {
                    self.chunk_writer.write_on_status_response(
                        response_level::STATUS,
                        response_code::NET_STREAM_PLAY_RESET,
                        "reset stream",
                        self.stream_properties.amf_version,
                    )?;
                }
                self.chunk_writer.write_on_status_response(
                    response_level::STATUS,
                    response_code::NET_STREAM_PLAY_START,
                    "play start",
                    self.stream_properties.amf_version,
                )?;
            }
        }

        self.flush_chunk().await?;
        Ok(())
    }

    fn process_play2_request(&mut self, request: Play2Command) -> RtmpServerResult<()> {
        todo!()
    }

    #[instrument]
    fn process_receive_audio_request(
        &mut self,
        request: ReceiveAudioCommand,
    ) -> RtmpServerResult<()> {
        match &mut self.runtime_handle {
            SessionRuntime::Play {
                stream_data_consumer: _,
                play_id: _,
                stream_type: _,
                receive_audio,
                receive_video: _,
                buffer_length: _,
            } => *receive_audio = request.bool_flag,
            _ => {
                tracing::warn!(
                    "got unexpected receive_audio request while not in play session: {:?}, ignore.",
                    request
                );
            }
        };
        Ok(())
    }

    #[instrument]
    fn process_receive_video_request(
        &mut self,
        request: ReceiveVideoCommand,
    ) -> RtmpServerResult<()> {
        match &mut self.runtime_handle {
            SessionRuntime::Play {
                stream_data_consumer: _,
                play_id: _,
                stream_type: _,
                receive_audio: _,
                receive_video,
                buffer_length: _,
            } => {
                *receive_video = request.bool_flag;
            }
            _ => {
                tracing::warn!(
                    "got unexpected receive_video request while not in play session: {:?}, ignore.",
                    request
                );
            }
        };
        Ok(())
    }

    fn process_seek_request(&mut self, request: SeekCommand) -> RtmpServerResult<()> {
        todo!()
    }

    async fn process_user_control_event(
        &mut self,
        request: UserControlEvent,
    ) -> RtmpServerResult<()> {
        match request {
            UserControlEvent::SetBufferLength {
                stream_id: _,
                buffer_length: len,
            } => match &mut self.runtime_handle {
                SessionRuntime::Play {
                    stream_data_consumer: _,
                    play_id: _,
                    stream_type: _,
                    receive_audio: _,
                    receive_video: _,
                    buffer_length,
                } => *buffer_length = Some(len),
                _ => {
                    tracing::warn!(
                        "got unexpected set_buffer_length event while not in a play session, event: {:?}, ignore",
                        request
                    );
                }
            },
            UserControlEvent::PingRequest { timestamp } => {
                tracing::trace!("got a ping request: {}", timestamp);
                self.chunk_writer.write_ping_response(timestamp)?;
                self.flush_chunk().await?;
            }
            UserControlEvent::PingResponse { timestamp } => {
                tracing::trace!("got a ping response: {}", timestamp);
            }
            _ => {
                tracing::warn!("got unexpected user control event: {:?}, ignore", request);
            }
        };
        Ok(())
    }
}
