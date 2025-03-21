use core::time;
use std::{
    backtrace::Backtrace,
    cmp::min,
    collections::HashMap,
    io::{self, Cursor},
    sync::Arc,
    time::{Duration, SystemTime},
};

use ::stream_center::{
    events::StreamCenterEvent,
    frame_info::{AggregateMeta, AudioMeta, ChunkFrameData, VideoMeta},
    stream_source::{StreamIdentifier, StreamType},
};
use rtmp_formats::{
    chunk::{
        self, ChunkMessage, ChunkMessageCommonHeader, RtmpChunkMessageBody,
        errors::ChunkMessageError,
    },
    commands::{
        CallCommandRequest, ConnectCommandRequest, ConnectCommandRequestObject,
        CreateStreamCommandRequest, DeleteStreamCommand, PauseCommand, Play2Command, PlayCommand,
        PublishCommand, ReceiveAudioCommand, ReceiveVideoCommand, RtmpC2SCommands, SeekCommand,
        consts::RESPONSE_STREAM_ID,
    },
    handshake,
    message::RtmpUserMessageBody,
    protocol_control::{
        AbortMessage, Acknowledgement, ProtocolControlMessage, SetChunkSize,
        SetPeerBandWidthLimitType, SetPeerBandwidth, WindowAckSize,
    },
    user_control::UserControlEvent,
};
use stream_center::{
    events::SubscribeResponse,
    frame_info::{MediaMessageRuntimeStat, ScriptMeta},
    gop::FLVMediaFrame,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufWriter},
    net::TcpStream,
    sync::{
        RwLock,
        mpsc::{self},
        oneshot,
    },
    time::timeout,
};
use tokio_util::{
    bytes::{Buf, BytesMut},
    either::Either,
};
use url::Url;
use utils::system::time::get_timestamp_ns;
use uuid::Uuid;

use crate::errors::RtmpServerError;

use super::{
    config::RtmpSessionConfig,
    consts::{response_code, response_level},
    errors::RtmpServerResult,
};

#[derive(Debug, Default)]
pub struct SessionStat {
    video_frame_cnt: u64,
    audio_frame_cnt: u64,
    script_frame_cnt: u64,
    aggregate_frame_cnt: u64,

    failed_video_frame_cnt: u64,
    failed_audio_frame_cnt: u64,
    failed_meta_frame_cnt: u64,
    failed_aggregate_frame_cnt: u64,
}

#[derive(Debug)]
struct PlayHandle {
    stream_data_consumer: mpsc::Receiver<FLVMediaFrame>,
    stream_type: StreamType,
    play_id: Uuid,
    receive_audio: bool,
    receive_video: bool,
    buffer_length: Option<u32>,
    stat: SessionStat,
}

#[derive(Debug)]
struct PublishHandle {
    stream_data_producer: mpsc::Sender<ChunkFrameData>,
    no_data_since: Option<SystemTime>,
    stat: SessionStat,
}

#[derive(Debug)]
enum SessionRuntime {
    Play(Arc<RwLock<PlayHandle>>),
    Publish(PublishHandle),
    Unknown,
}

#[derive(Debug, Default)]
struct StreamProperties {
    stream_name: String,
    app: String,

    stream_type: StreamType,

    stream_context: HashMap<String, String>,
}

#[derive(Debug)]
pub struct RtmpSession {
    read_buffer: BytesMut,
    stream: BufWriter<TcpStream>,
    chunk_reader: chunk::reader::Reader,
    chunk_writer: chunk::writer::Writer,

    runtime_handle: SessionRuntime,

    stream_properties: StreamProperties,
    connect_info: ConnectCommandRequestObject,

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
            connect_info: Default::default(),

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
            let mut buf = Cursor::new(&self.read_buffer);
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
                        self.ack_window_size(size).await?;
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
            log::error!("not flushable");
            return Ok(());
        }

        timeout(
            Duration::from_millis(self.config.write_timeout_ms),
            async move {
                self.chunk_writer.write_to(&mut self.stream).await?;
                self.stream.flush().await?;
                self.total_wrote_bytes = self.chunk_writer.get_bytes_written();
                Ok::<(), RtmpServerError>(())
            },
        )
        .await
        .map_err(|err| {
            RtmpServerError::Io(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("write chunk timeout, {}", err),
            ))
        })??;
        Ok(())
    }

    pub async fn run(&mut self) -> RtmpServerResult<()> {
        handshake::server::HandshakeServer::new(&mut self.stream)
            .handshake(false)
            .await?;
        self.chunk_writer.write_set_chunk_size(4096)?;

        loop {
            let play_handle = match &self.runtime_handle {
                SessionRuntime::Play(handle) => Some(handle.clone()),
                _ => None,
            };

            if let Some(play_handle) = play_handle {
                // let play_id = play_handle.read().await.play_id.clone();
                let res = self.playing(play_handle).await;
                match res {
                    Ok(_) => {
                        log::info!("play session successfully end");
                    }
                    Err(err) => {
                        log::info!("play session end with err: {:?}", err);
                    }
                }
                return Ok(());
            }

            match self.read_chunk().await {
                Ok(maybe_chunk) => match maybe_chunk {
                    Some(message) => {
                        self.process_message(message).await?;
                    }
                    None => {
                        if let SessionRuntime::Publish(handle) = &self.runtime_handle {
                            let current_time = SystemTime::now();
                            if current_time
                                .duration_since(handle.no_data_since.unwrap_or(current_time))
                                .expect("stop time must be before")
                                .as_secs()
                                > 10
                            {
                                // 10 seconds after publish stop, and no data received, we close this session
                                log::info!("publish session timeout, closing");
                                return Ok(());
                            }
                        }
                    }
                },
                Err(err) => match err {
                    RtmpServerError::ChunkMessageReadFailed(
                        ChunkMessageError::UnknownMessageType { type_id, backtrace },
                    ) => {
                        log::error!(
                            "got unknown message: type_id: {}, backtrace: {}",
                            type_id,
                            backtrace
                        );
                    }
                    RtmpServerError::Io(io_err) => {
                        if io_err.kind() == io::ErrorKind::ConnectionReset {
                            log::info!("connect reset by peer");
                            return Ok(());
                        }
                        log::error!("io error: {:?}", io_err);
                    }
                    err => {
                        log::error!("{:?}", err);
                        panic!();
                    }
                },
            }
        }
    }

    pub async fn clean_up(&self) -> RtmpServerResult<()> {
        match &self.runtime_handle {
            SessionRuntime::Play(play_handle) => {
                let play_id = play_handle.read().await.play_id;
                self.unsubscribe_from_stream_center(
                    play_id,
                    &self.stream_properties.stream_name,
                    &self.stream_properties.app,
                )
                .await?
            }
            SessionRuntime::Publish(_publish_handle) => {
                self.unpublish_from_stream_center(
                    &self.stream_properties.stream_name,
                    &self.stream_properties.app,
                )
                .await?
            }
            _ => {}
        }
        Ok(())
    }

    pub async fn log_stats(&self) {
        match &self.runtime_handle {
            SessionRuntime::Play(handle) => {
                let handle = handle.read().await;
                log::info!(
                    "play stats: stream_type: {}, play_id: {}, receive_audio: {}, receive_video: {}, buffer_length: {:?}, stat: {:?}, total_bytes written: {}",
                    handle.stream_type,
                    handle.play_id,
                    handle.receive_audio,
                    handle.receive_video,
                    handle.buffer_length,
                    handle.stat,
                    self.total_wrote_bytes
                )
            }
            SessionRuntime::Publish(handle) => {
                log::info!(
                    "publish stats: no_data_since: {:?}, stat: {:?}",
                    handle.no_data_since,
                    handle.stat
                );
            }
            _ => {}
        }
    }

    async fn playing(&mut self, play_handle: Arc<RwLock<PlayHandle>>) -> RtmpServerResult<()> {
        let mut messages = Vec::with_capacity(128);
        loop {
            let mut handle = play_handle.write().await;
            messages.clear();
            match handle
                .stream_data_consumer
                .recv_many(&mut messages, 128)
                .await
            {
                0 => {
                    log::error!("channel closed while trying to play");
                    return Err(RtmpServerError::StreamIsGone);
                }
                _len => {
                    for message in &mut messages {
                        match message {
                            FLVMediaFrame::Video {
                                runtime_stat,
                                pts,
                                header,
                                payload,
                            } => {
                                runtime_stat.play_time_ns = get_timestamp_ns().unwrap_or(0);
                                let timestamp =
                                    *pts + if let Some(cts) = header.composition_time {
                                        cts as u64
                                    } else {
                                        0
                                    } + if let Some(cts_nano) = header.timestamp_nano {
                                        cts_nano as u64
                                    } else {
                                        0
                                    };

                                let res = self
                                    .chunk_writer
                                    .write_video(payload.clone(), timestamp as u32);
                                if res.is_err() {
                                    log::error!(
                                        "write video message to rtmp chunk failed, err: {:?}",
                                        res
                                    );
                                    handle.stat.failed_video_frame_cnt += 1;
                                } else {
                                    handle.stat.video_frame_cnt += 1;
                                }
                            }
                            FLVMediaFrame::Audio {
                                runtime_stat,
                                pts,
                                header: _,
                                payload,
                            } => {
                                runtime_stat.play_time_ns = get_timestamp_ns().unwrap_or(0);

                                let res =
                                    self.chunk_writer.write_audio(payload.clone(), *pts as u32);
                                if res.is_err() {
                                    log::error!(
                                        "write audio message to rtmp chunk failed, err: {:?}",
                                        res
                                    );
                                    handle.stat.failed_audio_frame_cnt += 1;
                                } else {
                                    handle.stat.audio_frame_cnt += 1;
                                }
                            }
                            FLVMediaFrame::Script {
                                runtime_stat,
                                pts,
                                payload,
                                on_meta_data: _,
                            } => {
                                runtime_stat.play_time_ns = get_timestamp_ns().unwrap_or(0);

                                self.chunk_writer.write_meta(payload.clone(), *pts as u32)?;
                                //TODO -
                                handle.stat.script_frame_cnt += 1;
                            }
                        }
                        self.flush_chunk().await?;
                        // message.log_runtime_stat();
                    }
                }
            }
        }
    }

    async fn process_message(&mut self, message: ChunkMessage) -> RtmpServerResult<()> {
        let mut header = message.header;
        header.runtime_stat.process_time_ns = get_timestamp_ns().unwrap_or(0);

        let body = message.chunk_message_body;
        match body {
            RtmpChunkMessageBody::ProtocolControl(request) => {
                self.process_protocol_control_message(request).await?
            }
            RtmpChunkMessageBody::UserControl(control) => {
                self.process_user_control_event(control).await?
            }
            RtmpChunkMessageBody::RtmpUserMessage(message) => {
                self.process_user_message(*message, header).await?
            }
        }
        Ok(())
    }

    async fn process_user_message(
        &mut self,
        message: RtmpUserMessageBody,
        header: ChunkMessageCommonHeader,
    ) -> RtmpServerResult<()> {
        match message {
            RtmpUserMessageBody::C2SCommand(command) => {
                self.process_user_command(command, header).await?
            }
            RtmpUserMessageBody::MetaData { payload } => self.process_meta(header, payload).await?,
            RtmpUserMessageBody::Aggregate { payload } => {
                self.process_aggregate(header, payload).await?
            }
            RtmpUserMessageBody::Audio { payload } => self.process_audio(header, payload).await?,
            RtmpUserMessageBody::Video { payload } => self.process_video(header, payload).await?,
            RtmpUserMessageBody::S2Command(command) => {
                log::error!("got unexpected s2c command: {:?}", command);
            }
            RtmpUserMessageBody::SharedObject() => {
                log::warn!("ignore shared object command");
            }
        };
        Ok(())
    }

    async fn process_audio(
        &mut self,
        header: ChunkMessageCommonHeader,
        audio: BytesMut,
    ) -> RtmpServerResult<()> {
        match &mut self.runtime_handle {
            SessionRuntime::Publish(handle) => {
                handle.no_data_since = None;

                let res = handle
                    .stream_data_producer
                    .send(ChunkFrameData::Audio {
                        // we send the payload to stream center asap, parse and adjust meta info there
                        meta: AudioMeta {
                            pts: header.timestamp as u64,
                            runtime_stat: MediaMessageRuntimeStat {
                                read_time_ns: header.runtime_stat.read_time_ns,
                                session_process_time_ns: header.runtime_stat.process_time_ns,
                                publish_stream_source_time_ns: get_timestamp_ns().unwrap_or(0),
                                ..Default::default()
                            },
                        },
                        payload: audio,
                    })
                    .await
                    .map_err(|err| {
                        log::error!("send audio data to stream center failed: {:?}", err);
                        RtmpServerError::ChannelSendFailed {
                            backtrace: Backtrace::capture(),
                        }
                    });
                if res.is_err() {
                    handle.stat.failed_audio_frame_cnt += 1;
                } else {
                    handle.stat.audio_frame_cnt += 1;
                }
                res?;
            }
            _ => {
                log::error!(
                    "got audio data while stream not published, length: {}",
                    audio.len()
                );
            }
        };
        Ok(())
    }

    async fn process_video(
        &mut self,
        header: ChunkMessageCommonHeader,
        video: BytesMut,
    ) -> RtmpServerResult<()> {
        match &mut self.runtime_handle {
            SessionRuntime::Publish(handle) => {
                handle.no_data_since = None;

                let res = handle
                    .stream_data_producer
                    .send(ChunkFrameData::Video {
                        meta: VideoMeta {
                            pts: header.timestamp as u64,
                            runtime_stat: MediaMessageRuntimeStat {
                                read_time_ns: header.runtime_stat.read_time_ns,
                                session_process_time_ns: header.runtime_stat.process_time_ns,
                                publish_stream_source_time_ns: get_timestamp_ns().unwrap_or(0),
                                ..Default::default()
                            },
                        },
                        payload: video,
                    })
                    .await
                    .map_err(|err| {
                        log::error!("send video to stream center failed: {:?}", err);
                        RtmpServerError::ChannelSendFailed {
                            backtrace: Backtrace::capture(),
                        }
                    });
                if res.is_err() {
                    handle.stat.failed_video_frame_cnt += 1;
                } else {
                    handle.stat.video_frame_cnt += 1;
                }
                res?;
            }
            _ => {
                log::error!(
                    "got video data while stream not published, length: {}",
                    video.len()
                );
            }
        };
        Ok(())
    }

    async fn process_meta(
        &mut self,
        header: ChunkMessageCommonHeader,
        payload: BytesMut,
    ) -> RtmpServerResult<()> {
        match &mut self.runtime_handle {
            SessionRuntime::Publish(handle) => {
                handle.no_data_since = None;

                let res = handle
                    .stream_data_producer
                    .send(ChunkFrameData::Script {
                        meta: ScriptMeta {
                            pts: header.timestamp as u64,
                            runtime_stat: MediaMessageRuntimeStat {
                                read_time_ns: header.runtime_stat.read_time_ns,
                                session_process_time_ns: header.runtime_stat.process_time_ns,
                                publish_stream_source_time_ns: get_timestamp_ns().unwrap_or(0),
                                ..Default::default()
                            },
                        },
                        payload,
                    })
                    .await
                    .map_err(|err| {
                        log::error!("send meta to stream center failed: {:?}", err);
                        RtmpServerError::ChannelSendFailed {
                            backtrace: Backtrace::capture(),
                        }
                    });
                if res.is_err() {
                    handle.stat.failed_meta_frame_cnt += 1;
                } else {
                    handle.stat.script_frame_cnt += 1;
                }
                res?;
            }
            _ => {
                log::error!(
                    "got meta data while stream not published, value: {:?}",
                    payload
                );
            }
        };
        Ok(())
    }

    async fn process_aggregate(
        &mut self,
        header: ChunkMessageCommonHeader,
        aggregate: BytesMut,
    ) -> RtmpServerResult<()> {
        match &mut self.runtime_handle {
            SessionRuntime::Publish(handle) => {
                handle.no_data_since = None;

                let res = handle
                    .stream_data_producer
                    .send(ChunkFrameData::Aggregate {
                        meta: AggregateMeta {
                            pts: header.timestamp as u64,
                            read_time_ns: header.runtime_stat.read_time_ns,
                            session_process_time_ns: header.runtime_stat.process_time_ns,
                            publish_stream_source_time_ns: get_timestamp_ns().unwrap_or(0),
                        },
                        data: aggregate,
                    })
                    .await
                    .map_err(|_err| RtmpServerError::ChannelSendFailed {
                        backtrace: Backtrace::capture(),
                    });

                if res.is_err() {
                    handle.stat.failed_aggregate_frame_cnt += 1;
                } else {
                    handle.stat.aggregate_frame_cnt += 1;
                }
                res?;
            }
            _ => {
                log::error!(
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
                self.process_receive_audio_request(request).await?
            }
            RtmpC2SCommands::ReceiveVideo(request) => {
                self.process_receive_video_request(request).await?
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

        self.stream_properties.app = request.command_object.app.clone();

        self.connect_info = request.command_object;

        self.chunk_writer.write_connect_response(
            true,
            request.transaction_id.into(),
            super::consts::FMSVER,
            super::consts::FMS_CAPABILITIES,
            super::consts::response_code::NET_CONNECTION_CONNECT_SUCCESS,
            super::consts::response_level::STATUS,
            "Connection Succeeded.",
            self.connect_info.object_encoding,
        )?;
        self.flush_chunk().await?;

        log::info!("connect done, connect_info: {:?}", self.connect_info,);

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
            self.connect_info.object_encoding,
            None,
        )?;
        self.flush_chunk().await?;

        log::info!("process publish command success");
        Ok(())
    }

    // for enhanced rtmp reconnect command, might not be useful
    #[allow(dead_code)]
    async fn write_reconnect_command(
        &mut self,
        new_tc_url: &str,
        description: Option<&str>,
    ) -> RtmpServerResult<()> {
        let mut tc_url_arg = HashMap::new();
        tc_url_arg.insert(
            "tcUrl".to_string(),
            amf::string(new_tc_url, self.connect_info.object_encoding),
        );
        self.chunk_writer.write_on_status_response(
            response_level::STATUS,
            response_code::NET_CONNECTION_CONNECT_RECONNECT_REQUEST,
            description.unwrap_or("The streaming server is undergoing updates."),
            self.connect_info.object_encoding,
            Some(tc_url_arg),
        )?;
        self.flush_chunk().await?;
        Ok(())
    }

    async fn unpublish_from_stream_center(
        &self,
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
                log::error!(
                    "send unpublish event to stream center failed, {:?}. stream_name: {}, app: {}",
                    err,
                    stream_name,
                    app
                );
                RtmpServerError::ChannelSendFailed {
                    backtrace: Backtrace::capture(),
                }
            })?;

        match rx.await {
            Err(_err) => {
                log::error!("channel closed while trying to receive unpublish result");
                return Err(RtmpServerError::ChannelSendFailed {
                    backtrace: Backtrace::capture(),
                });
            }
            Ok(Err(err)) => {
                log::error!(
                    "stream unpublish from stream center failed, {:?}. stream_name: {}, app: {}",
                    err,
                    stream_name,
                    app
                );
            }
            Ok(Ok(())) => {
                log::info!(
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
        log::info!("process delete stream command, request: {:?}", request);
        let _ = self
            .unpublish_from_stream_center(
                &self.stream_properties.stream_name.clone(),
                &self.stream_properties.app.clone(),
            )
            .await;

        self.chunk_writer.write_on_status_response(
            response_level::STATUS,
            response_code::NET_STREAM_DELETE_SUCCESS,
            "delete stream success",
            self.connect_info.object_encoding,
            None,
        )?;

        self.flush_chunk().await?;

        log::info!("process delete stream command success");
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
        log::trace!(
            "update read chunk size, from {} to {}",
            old_size,
            chunk_size
        );
    }

    fn process_abort_chunk_request(&mut self, request: AbortMessage) {
        log::info!("got abort request: {:?}", request);
        self.chunk_reader
            .abort_chunk_message(request.chunk_stream_id);
    }

    fn process_window_ack_size_request(&mut self, request: WindowAckSize) {
        log::info!("got window_ack_size request: {:?}", request);
        self.ack_window_size_read = Some(request.size);
    }

    async fn ack_window_size(&mut self, size: u32) -> RtmpServerResult<()> {
        log::info!("do ack: {}", size);
        self.chunk_writer.write_acknowledgement_message(size)?;
        self.flush_chunk().await?;
        Ok(())
    }

    async fn process_set_peer_bandwidth_request(
        &mut self,
        request: SetPeerBandwidth,
    ) -> RtmpServerResult<()> {
        log::info!("got set_peer_bandwidth request: {:?}", request);
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
                        log::trace!(
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
        log::info!("got acknowledge request: {:?}", request);
        self.acknowledged_sequence_number = Some(request.sequence_number);
    }

    async fn publish_to_stream_center(
        &mut self,
        stream_name: &str,
        stream_type: StreamType,
    ) -> RtmpServerResult<()> {
        if let SessionRuntime::Publish(_) = self.runtime_handle {
            return Ok(());
        }

        if stream_name.is_empty() {
            return Err(RtmpServerError::InvalidStreamParam(
                "stream publish need at least stream_name, got empty".to_owned(),
            ));
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
                log::error!(
                    "send publish event to stream center failed, {:?}. stream_name: {}, app: {}",
                    err,
                    stream_name,
                    self.stream_properties.app.clone(),
                );
                RtmpServerError::ChannelSendFailed {
                    backtrace: Backtrace::capture(),
                }
            })?;

        match rx.await {
            Err(_err) => {
                log::error!("channel closed while trying to receive publish result");
                return Err(RtmpServerError::ChannelSendFailed {
                    backtrace: Backtrace::capture(),
                });
            }
            Ok(Err(err)) => {
                log::error!(
                    "publish to stream center failed, {:?}. stream_name: {},  app: {}, stream_type: {}",
                    err,
                    self.stream_properties.stream_name,
                    self.stream_properties.app,
                    self.stream_properties.stream_type,
                );
                return Err(err.into());
            }
            Ok(Ok(sender)) => {
                self.runtime_handle = SessionRuntime::Publish(PublishHandle {
                    stream_data_producer: sender,
                    no_data_since: None,
                    stat: Default::default(),
                });
                log::info!(
                    "publish to stream center success, stream_name: {}, app: {}",
                    self.stream_properties.stream_name,
                    self.stream_properties.app
                );
            }
        }

        Ok(())
    }

    async fn process_call_request(&mut self, request: CallCommandRequest) -> RtmpServerResult<()> {
        log::info!("process call request: {:?}", request);
        let command_name = &request.procedure_name;
        match command_name.as_str() {
            "releaseStream" | "FCPublish" | "FCUnpublish" => {
                //NOTE - publish start command, same as createStream or publish
                let stream_name = match &request.optional_arguments {
                    None => None,
                    Some(v) => match v {
                        Either::Left(amf_any) => amf_any.try_as_str().map_or_else(|| None, Some),
                        Either::Right(_map) => None,
                    },
                };

                match stream_name {
                    None => {
                        log::warn!(
                            "ignore call request as no stream name provided. {:?}",
                            request
                        );
                    }
                    Some(stream_name) => {
                        // ignore the result
                        let res = if command_name == "FCUnpublish" {
                            let _res = self
                                .unpublish_from_stream_center(
                                    &self.stream_properties.stream_name.clone(),
                                    &self.stream_properties.app.clone(),
                                )
                                .await;
                            // we do not care the unpublish result
                            Ok(())
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
                log::warn!("got a call request, ignore. request: {:?}", request);
            }
        }
        Ok(())
    }

    fn process_pause_request(&mut self, request: PauseCommand) -> RtmpServerResult<()> {
        log::warn!("got a pause request, ignore. request: {:?}", request);
        Ok(())
    }

    async fn subscribe_from_stream_center(
        &self,
        stream_name: &str,
        app: &str,
        context: HashMap<String, String>,
    ) -> RtmpServerResult<SubscribeResponse> {
        let (tx, rx) = oneshot::channel();
        self.stream_center_event_sender
            .send(StreamCenterEvent::Subscribe {
                stream_id: StreamIdentifier {
                    stream_name: stream_name.to_string(),
                    app: app.to_string(),
                },
                context,
                result_sender: tx,
            })
            .map_err(|err| {
                log::error!(
                    "send subscribe event to stream center failed, {:?}. stream_name: {}, app: {}",
                    err,
                    stream_name,
                    app
                );
                RtmpServerError::ChannelSendFailed {
                    backtrace: Backtrace::capture(),
                }
            })?;

        match rx.await {
            Err(_err) => {
                log::error!(
                    "channel closed while trying receive subscribe result, stream_name: {}, app: {}",
                    stream_name,
                    app
                );
                Err(RtmpServerError::ChannelSendFailed {
                    backtrace: Backtrace::capture(),
                })
            }
            Ok(Err(err)) => {
                log::error!(
                    "subscribe from stream center failed, {:?}, stream_name: {}, app: {}",
                    err,
                    stream_name,
                    app
                );
                Err(err.into())
            }
            Ok(Ok(response)) => {
                log::info!(
                    "subscribe from stream center success, stream_name: {}, app: {}",
                    stream_name,
                    app
                );
                Ok(response)
            }
        }
    }

    async fn unsubscribe_from_stream_center(
        &self,
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
                log::error!(
                    "send unsubscribe event to stream center failed, {:?}. stream_name: {}, app: {}, uuid: {}",
                    err,
                    stream_name,
                    app,
                    uuid,
                );
                RtmpServerError::ChannelSendFailed {
                    backtrace: Backtrace::capture(),
                }
            })?;

        match rx.await {
            Err(_err) => {
                log::error!("channel closed while trying to receive unsubscribe result");
                return Err(RtmpServerError::ChannelSendFailed {
                    backtrace: Backtrace::capture(),
                });
            }
            Ok(Err(err)) => {
                log::error!(
                    "unsubscribe from stream center failed, uuid: {}, stream_name: {}, app: {}",
                    uuid,
                    stream_name,
                    app
                );
                return Err(err.into());
            }
            Ok(Ok(())) => {
                log::info!(
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
        log::info!("got play request: {:?}", request);
        let stream_play_path = format!("rtmp://fake_host/{}", request.stream_name);
        let url = Url::parse(&stream_play_path).map_err(|err| {
            RtmpServerError::InvalidStreamParam(format!(
                "stream play code parse failed: {:?}, should be url path format, got: {}",
                err, request.stream_name
            ))
        })?;

        let stream_name = url.path_segments();
        if stream_name.is_none() {
            return Err(RtmpServerError::InvalidStreamParam(format!(
                "stream play code parse failed, no stream_name: {}",
                request.stream_name
            )));
        }
        let stream_name = stream_name.unwrap().collect::<Vec<&str>>().first().cloned();
        if stream_name.is_none() {
            return Err(RtmpServerError::InvalidStreamParam(format!(
                "stream play code parse failed, no stream_name: {}",
                request.stream_name
            )));
        }
        let stream_name = stream_name.unwrap();
        for (k, v) in url.query_pairs() {
            self.stream_properties
                .stream_context
                .insert(k.to_string(), v.to_string());
        }

        let _start = request.start; // this might by useful
        let _duration = request.duration; // this might by useful
        let reset = request.reset; // this should be ignored

        self.stream_properties.stream_name = stream_name.to_string();
        let subscribe_result = self
            .subscribe_from_stream_center(
                stream_name,
                &self.stream_properties.app.clone(),
                self.stream_properties.stream_context.clone(),
            )
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
                log::error!("subscribe stream failed: {:?}", err);
                self.chunk_writer.write_on_status_response(
                    response_level::ERROR,
                    response_code::NET_STREAM_PLAY_NOT_FOUND,
                    "stream not found",
                    self.connect_info.object_encoding,
                    None,
                )?;
            }
            Ok(response) => {
                self.runtime_handle = SessionRuntime::Play(Arc::new(RwLock::new(PlayHandle {
                    stream_data_consumer: response.media_receiver,
                    stream_type: response.stream_type,
                    receive_audio: response.has_audio,
                    receive_video: response.has_video,
                    buffer_length: None,
                    play_id: response.subscribe_id,
                    stat: Default::default(),
                })));
                if reset {
                    self.chunk_writer.write_on_status_response(
                        response_level::STATUS,
                        response_code::NET_STREAM_PLAY_RESET,
                        "reset stream",
                        self.connect_info.object_encoding,
                        None,
                    )?;
                }
                self.chunk_writer.write_on_status_response(
                    response_level::STATUS,
                    response_code::NET_STREAM_PLAY_START,
                    "play start",
                    self.connect_info.object_encoding,
                    None,
                )?;
            }
        }

        self.flush_chunk().await?;
        Ok(())
    }

    fn process_play2_request(&mut self, _request: Play2Command) -> RtmpServerResult<()> {
        todo!()
    }

    async fn process_receive_audio_request(
        &mut self,
        request: ReceiveAudioCommand,
    ) -> RtmpServerResult<()> {
        match &mut self.runtime_handle {
            SessionRuntime::Play(handle) => handle.write().await.receive_audio = request.bool_flag,
            _ => {
                log::warn!(
                    "got unexpected receive_audio request while not in play session: {:?}, ignore.",
                    request
                );
            }
        };
        Ok(())
    }

    async fn process_receive_video_request(
        &mut self,
        request: ReceiveVideoCommand,
    ) -> RtmpServerResult<()> {
        match &mut self.runtime_handle {
            SessionRuntime::Play(handle) => {
                handle.write().await.receive_video = request.bool_flag;
            }
            _ => {
                log::warn!(
                    "got unexpected receive_video request while not in play session: {:?}, ignore.",
                    request
                );
            }
        };
        Ok(())
    }

    fn process_seek_request(&mut self, _request: SeekCommand) -> RtmpServerResult<()> {
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
                SessionRuntime::Play(handle) => handle.write().await.buffer_length = Some(len),
                _ => {
                    log::warn!(
                        "got unexpected set_buffer_length event while not in a play session, event: {:?}, ignore",
                        request
                    );
                }
            },
            UserControlEvent::PingRequest { timestamp } => {
                log::trace!("got a ping request: {}", timestamp);
                self.chunk_writer.write_ping_response(timestamp)?;
                self.flush_chunk().await?;
            }
            UserControlEvent::PingResponse { timestamp } => {
                log::trace!("got a ping response: {}", timestamp);
            }
            _ => {
                log::warn!("got unexpected user control event: {:?}, ignore", request);
            }
        };
        Ok(())
    }
}
