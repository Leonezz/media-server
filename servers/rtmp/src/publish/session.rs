use core::time;
use std::{
    backtrace::Backtrace,
    cmp::min,
    collections::HashMap,
    io::{self, Cursor},
    time::SystemTime,
};

use ::stream_center::{
    frame_info::{AggregateMeta, AudioMeta, FrameData, VideoMeta},
    stream_source::StreamType,
};
use rtmp_formats::{
    chunk::{self, ChunkMessage, RtmpChunkMessageBody, errors::ChunkMessageError},
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
    sync::{broadcast, mpsc},
};
use tokio_util::{
    bytes::{Buf, BytesMut},
    either::Either,
};
use tracing::instrument;

use crate::publish::errors::RtmpPublishServerError;

use super::{
    config::RtmpSessionConfig,
    consts::{response_code, response_level},
    errors::RtmpPublishServerResult,
};

#[derive(Debug)]
enum SessionRuntime {
    Play {
        stream_data_consumer: broadcast::Receiver<FrameData>,
        stream_type: StreamType,
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
pub struct RtmpPublishSession {
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
}

impl RtmpPublishSession {
    pub fn new(io: TcpStream, config: RtmpSessionConfig) -> Self {
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
        }
    }

    async fn read_chunk(&mut self) -> RtmpPublishServerResult<Option<ChunkMessage>> {
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
                time::Duration::from_secs(10),
                self.stream.read_buf(&mut self.read_buffer),
            )
            .await
            {
                Ok(len) => match len {
                    Ok(len) => {
                        if len == 0 {
                            if self.read_buffer.is_empty() {
                                return Ok(None);
                            } else {
                                return Err(RtmpPublishServerError::Io(io::Error::new(
                                    io::ErrorKind::ConnectionReset,
                                    "connect reset by peer",
                                )));
                            }
                        }
                    }
                    Err(err) => return Err(err.into()),
                },
                Err(err) => {
                    return Err(RtmpPublishServerError::Io(io::Error::new(
                        io::ErrorKind::TimedOut,
                        format!("read chunk data timeout: {}", err),
                    )));
                }
            }
        }
    }

    async fn flush_chunk(&mut self) -> RtmpPublishServerResult<()> {
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
        self.stream.flush().await?;
        self.total_wrote_bytes = self.chunk_writer.get_bytes_written();
        Ok(())
    }

    pub async fn run(&mut self) -> RtmpPublishServerResult<()> {
        handshake::server::HandshakeServer::new(&mut self.stream)
            .handshake(false)
            .await?;
        self.chunk_writer.write_set_chunk_size(4096)?;

        scope_guard::scope_guard!(|| {
            match self.runtime_handle {
                SessionRuntime::Publish {
                    stream_data_producer: _,
                    no_data_since: _,
                } => {
                    let stream_name = self.stream_properties.stream_name.clone();
                    let app = self.stream_properties.app.clone();
                    tokio::task::spawn_blocking(move || {
                        Handle::current().block_on(async move {
                            if let Err(err) = stream_center::unpublish(&stream_name, &app).await {
                                tracing::error!("failed to unpublish stream: {}", err);
                            }
                        })
                    });
                }
                _ => {}
            }
        });

        loop {
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
                    RtmpPublishServerError::ChunkMessageReadFailed(
                        ChunkMessageError::UnknownMessageType { type_id, backtrace },
                    ) => {}
                    RtmpPublishServerError::Io(io_err) => {
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

    async fn process_message(&mut self, message: ChunkMessage) -> RtmpPublishServerResult<()> {
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
                self.process_user_message(message).await?
            }
        }
        Ok(())
    }

    async fn process_user_message(
        &mut self,
        message: RtmpUserMessageBody,
    ) -> RtmpPublishServerResult<()> {
        let _ = match message {
            RtmpUserMessageBody::C2SCommand(command) => self.process_user_command(command).await?,
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

    async fn process_audio(&mut self, audio: BytesMut) -> RtmpPublishServerResult<()> {
        let _ = match &mut self.runtime_handle {
            SessionRuntime::Publish {
                stream_data_producer,
                no_data_since: _,
            } => stream_data_producer
                .send(FrameData::Audio {
                    meta: AudioMeta::default(),
                    data: audio,
                })
                .await
                .map_err(|err| RtmpPublishServerError::ChannelSendFailed {
                    backtrace: Backtrace::capture(),
                }),
            _ => {
                tracing::error!(
                    "got audio data while stream not published, length: {}",
                    audio.len()
                );
                Ok(())
            }
        };
        Ok(())
    }

    async fn process_video(&mut self, video: BytesMut) -> RtmpPublishServerResult<()> {
        let _ = match &mut self.runtime_handle {
            SessionRuntime::Publish {
                stream_data_producer,
                no_data_since: _,
            } => stream_data_producer
                .send(FrameData::Video {
                    meta: VideoMeta::default(),
                    data: video,
                })
                .await
                .map_err(|err| {
                    tracing::error!("send video to stream center failed: {:?}", err);
                    RtmpPublishServerError::ChannelSendFailed {
                        backtrace: Backtrace::capture(),
                    }
                }),
            _ => {
                tracing::error!(
                    "got video data while stream not published, length: {}",
                    video.len()
                );
                Ok(())
            }
        };
        Ok(())
    }

    async fn process_meta(&mut self, meta: amf::Value) -> RtmpPublishServerResult<()> {
        //TODO -
        Ok(())
    }

    async fn process_aggregate(&mut self, aggregate: BytesMut) -> RtmpPublishServerResult<()> {
        let _ = match &mut self.runtime_handle {
            SessionRuntime::Publish {
                stream_data_producer,
                no_data_since: _,
            } => stream_data_producer
                .send(FrameData::Aggregate {
                    meta: AggregateMeta::default(),
                    data: aggregate,
                })
                .await
                .map_err(|err| RtmpPublishServerError::ChannelSendFailed {
                    backtrace: Backtrace::capture(),
                }),
            _ => {
                tracing::error!(
                    "got aggregate data while stream not published, length: {}",
                    aggregate.len()
                );
                Ok(())
            }
        };
        Ok(())
    }

    async fn process_user_command(
        &mut self,
        command: RtmpC2SCommands,
    ) -> RtmpPublishServerResult<()> {
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
            RtmpC2SCommands::Play(request) => self.process_play_request(request).await?,
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

    #[instrument]
    async fn process_connect_command(
        &mut self,
        request: ConnectCommandRequest,
    ) -> RtmpPublishServerResult<()> {
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

    #[instrument]
    async fn process_create_stream_command(
        &mut self,
        request: CreateStreamCommandRequest,
    ) -> RtmpPublishServerResult<()> {
        self.chunk_writer.write_create_stream_response(
            true,
            request.transaction_id,
            None,
            RESPONSE_STREAM_ID.into(),
        )?;
        self.flush_chunk().await?;
        Ok(())
    }

    #[instrument]
    async fn process_publish_command(
        &mut self,
        request: PublishCommand,
    ) -> RtmpPublishServerResult<()> {
        let stream_type: StreamType = request.publishing_type.try_into()?;
        self.publish_to_stream_center(&request.publishing_name, stream_type)?;

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

    #[instrument]
    async fn process_delete_stream_command(
        &mut self,
        request: DeleteStreamCommand,
    ) -> RtmpPublishServerResult<()> {
        let _ = stream_center::unpublish(
            &self.stream_properties.stream_name,
            &self.stream_properties.app,
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

    #[instrument]
    async fn process_protocol_control_message(
        &mut self,
        request: ProtocolControlMessage,
    ) -> RtmpPublishServerResult<()> {
        match request {
            ProtocolControlMessage::SetChunkSize(request) => {
                self.process_set_chunk_size_request(request).await?
            }
            ProtocolControlMessage::Abort(request) => self.process_abort_chunk_request(request)?,
            ProtocolControlMessage::Ack(request) => {
                self.process_acknowledge_request(request);
            }
            ProtocolControlMessage::SetPeerBandwidth(request) => {
                self.process_set_peer_bandwidth_request(request).await?;
            }
            ProtocolControlMessage::WindowAckSize(request) => {
                self.process_window_ack_size_request(request)?
            }
        }
        Ok(())
    }

    #[instrument]
    async fn process_set_chunk_size_request(
        &mut self,
        request: SetChunkSize,
    ) -> RtmpPublishServerResult<()> {
        let chunk_size = request.chunk_size;
        let old_size = self.chunk_reader.set_chunk_size(chunk_size as usize);
        tracing::trace!(
            "update read chunk size, from {} to {}",
            old_size,
            chunk_size
        );
        Ok(())
    }

    #[instrument]
    fn process_abort_chunk_request(
        &mut self,
        request: AbortMessage,
    ) -> RtmpPublishServerResult<()> {
        tracing::info!("got abort request: {:?}", request);
        self.chunk_reader
            .abort_chunk_message(request.chunk_stream_id);
        Ok(())
    }

    #[instrument]
    fn process_window_ack_size_request(
        &mut self,
        request: WindowAckSize,
    ) -> RtmpPublishServerResult<()> {
        tracing::info!("got window_ack_size request: {:?}", request);
        self.ack_window_size_read = Some(request.size);
        Ok(())
    }

    #[instrument]
    async fn ack_window_size(&mut self, size: u32) -> RtmpPublishServerResult<()> {
        tracing::info!("do ack: {}", size);
        self.chunk_writer.write_acknowledgement_message(size)?;
        self.flush_chunk().await?;
        Ok(())
    }

    #[instrument]
    async fn process_set_peer_bandwidth_request(
        &mut self,
        request: SetPeerBandwidth,
    ) -> RtmpPublishServerResult<()> {
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

    #[instrument]
    fn process_acknowledge_request(&mut self, request: Acknowledgement) {
        tracing::info!("got acknowledge request: {:?}", request);
        self.acknowledged_sequence_number = Some(request.sequence_number);
    }

    fn publish_to_stream_center(
        &mut self,
        stream_name: &str,
        stream_type: StreamType,
    ) -> RtmpPublishServerResult<()> {
        match self.runtime_handle {
            SessionRuntime::Publish {
                stream_data_producer: _,
                no_data_since: _,
            } => return Ok(()),
            _ => {}
        };

        if stream_name.is_empty() {
            return Err(RtmpPublishServerError::InvalidStreamParam);
        }

        self.stream_properties.stream_name = stream_name.to_string();
        self.stream_properties.stream_type = stream_type;

        let sender = stream_center::publish(
            &self.stream_properties.stream_name,
            &self.stream_properties.app,
            self.stream_properties.stream_type,
            self.stream_properties.stream_context.clone(),
        )?;

        self.runtime_handle = SessionRuntime::Publish {
            stream_data_producer: sender,
            no_data_since: None,
        };
        Ok(())
    }

    #[instrument]
    async fn process_call_request(
        &mut self,
        request: CallCommandRequest,
    ) -> RtmpPublishServerResult<()> {
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
                            let res = stream_center::unpublish(
                                &self.stream_properties.stream_name,
                                &self.stream_properties.app,
                            )
                            .await;
                            self.runtime_handle = SessionRuntime::PublishStop {
                                stop_time: SystemTime::now(),
                            };
                            res.map_err(|err| err.into())
                        } else {
                            self.publish_to_stream_center(stream_name, StreamType::default())
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

    fn process_pause_request(&mut self, request: PauseCommand) -> RtmpPublishServerResult<()> {
        tracing::warn!("got a pause request, ignore. request: {:?}", request);
        Ok(())
    }

    #[instrument]
    async fn process_play_request(&mut self, request: PlayCommand) -> RtmpPublishServerResult<()> {
        tracing::info!("got play request: {:?}", request);
        let stream_name = request.stream_name;
        let start = request.start; // this might by useful
        let duration = request.duration; // this might by useful
        let reset = request.reset; // this should be ignored

        let subscribe_res =
            stream_center::subscribe(&stream_name, &self.stream_properties.app).await;

        self.chunk_writer
            .write_set_chunk_size(self.config.chunk_size)?;
        if self.stream_properties.stream_type == StreamType::Record {
            self.chunk_writer.write_stream_ids_recorded(0)?; // I bet the stream_id is useless
        }

        self.chunk_writer.write_stream_begin(0)?;
        match subscribe_res {
            Err(err) => {
                tracing::error!("subscribe stream failed: {:?}", err);
                self.chunk_writer.write_on_status_response(
                    response_level::ERROR,
                    response_code::NET_STREAM_PLAY_NOT_FOUND,
                    "stream not found",
                    self.stream_properties.amf_version,
                )?;
            }
            Ok((receiver, stream_type)) => {
                self.runtime_handle = SessionRuntime::Play {
                    stream_data_consumer: receiver,
                    stream_type,
                    receive_audio: true,
                    receive_video: true,
                    buffer_length: None,
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

    fn process_play2_request(&mut self, request: Play2Command) -> RtmpPublishServerResult<()> {
        todo!()
    }

    #[instrument]
    fn process_receive_audio_request(
        &mut self,
        request: ReceiveAudioCommand,
    ) -> RtmpPublishServerResult<()> {
        match &mut self.runtime_handle {
            SessionRuntime::Play {
                stream_data_consumer: _,
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
    ) -> RtmpPublishServerResult<()> {
        match &mut self.runtime_handle {
            SessionRuntime::Play {
                stream_data_consumer: _,
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

    fn process_seek_request(&mut self, request: SeekCommand) -> RtmpPublishServerResult<()> {
        todo!()
    }

    #[instrument]
    async fn process_user_control_event(
        &mut self,
        request: UserControlEvent,
    ) -> RtmpPublishServerResult<()> {
        match request {
            UserControlEvent::SetBufferLength {
                stream_id: _,
                buffer_length: len,
            } => match &mut self.runtime_handle {
                SessionRuntime::Play {
                    stream_data_consumer: _,
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
