use std::{
    backtrace::Backtrace,
    collections::HashMap,
    io::{self, Cursor, Read},
    sync::Arc,
    time::SystemTime,
};

use ::stream_center::{
    events::StreamCenterEvent,
    frame_info::{AggregateMeta, AudioMeta, VideoMeta},
    stream_source::{StreamIdentifier, StreamType},
};
use flv::tag::{FLVTagType, on_meta_data::OnMetaData};
use rtmp_formats::{
    chunk::{
        ChunkMessage, ChunkMessageCommonHeader, RtmpChunkMessageBody, errors::ChunkMessageError,
    },
    commands::{
        CallCommandRequest, ConnectCommandRequest, ConnectCommandRequestObject,
        CreateStreamCommandRequest, DeleteStreamCommand, PauseCommand, Play2Command, PlayCommand,
        PublishCommand, ReceiveAudioCommand, ReceiveVideoCommand, RtmpC2SCommands, SeekCommand,
        consts::RESPONSE_STREAM_ID,
    },
    message::RtmpUserMessageBody,
    protocol_control::SetPeerBandWidthLimitType,
    user_control::UserControlEvent,
};
use stream_center::{
    events::SubscribeResponse,
    frame_info::{MediaMessageRuntimeStat, ScriptMeta},
    gop::MediaFrame,
};
use tokio::{
    net::TcpStream,
    sync::{
        RwLock,
        mpsc::{self},
        oneshot,
    },
};
use tokio_util::{
    bytes::{Buf, BytesMut},
    either::Either,
};
use url::Url;
use utils::system::time::get_timestamp_ns;
use uuid::Uuid;

use crate::{chunk_stream::RtmpChunkStream, errors::RtmpServerError};

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
    stream_data_consumer: mpsc::Receiver<MediaFrame>,
    stream_type: StreamType,
    play_id: Uuid,
    receive_audio: bool,
    receive_video: bool,
    buffer_length: Option<u32>,
    stat: SessionStat,
}

#[derive(Debug)]
struct PublishHandle {
    stream_data_producer: mpsc::Sender<MediaFrame>,
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
    chunk_stream: RtmpChunkStream,
    runtime_handle: SessionRuntime,
    stream_properties: StreamProperties,
    connect_info: ConnectCommandRequestObject,
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
            chunk_stream: RtmpChunkStream::new(
                4096,
                io,
                config.chunk_size,
                config.read_timeout_ms,
                config.write_timeout_ms,
            ),
            stream_properties: StreamProperties::default(),
            connect_info: Default::default(),
            runtime_handle: SessionRuntime::Unknown,
            total_wrote_bytes: 0,
            config,
            stream_center_event_sender,
        }
    }

    pub async fn run(&mut self) -> RtmpServerResult<()> {
        self.chunk_stream.handshake().await?;

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
                        tracing::info!("play session successfully end");
                    }
                    Err(err) => {
                        tracing::info!("play session end with err: {:?}", err);
                    }
                }
                return Ok(());
            }

            match self.chunk_stream.read_chunk().await {
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
                                tracing::info!("publish session timeout, closing");
                                return Ok(());
                            }
                        }
                    }
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
                        if io_err.kind() == io::ErrorKind::WouldBlock {
                            continue;
                        }
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
                tracing::info!(
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
                tracing::info!(
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
                    tracing::error!("channel closed while trying to play");
                    return Err(RtmpServerError::StreamIsGone);
                }
                _len => {
                    for message in &mut messages {
                        match message {
                            MediaFrame::Video {
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

                                let res = self.chunk_stream.write_tag(message).await;
                                if res.is_err() {
                                    tracing::error!(
                                        "write video message to rtmp chunk failed, err: {:?}",
                                        res
                                    );
                                    handle.stat.failed_video_frame_cnt += 1;
                                } else {
                                    handle.stat.video_frame_cnt += 1;
                                }
                                res?
                            }
                            MediaFrame::Audio {
                                runtime_stat,
                                pts,
                                header: _,
                                payload,
                            } => {
                                runtime_stat.play_time_ns = get_timestamp_ns().unwrap_or(0);

                                let res = self.chunk_stream.write_tag(message).await;
                                if res.is_err() {
                                    tracing::error!(
                                        "write audio message to rtmp chunk failed, err: {:?}",
                                        res
                                    );
                                    handle.stat.failed_audio_frame_cnt += 1;
                                } else {
                                    handle.stat.audio_frame_cnt += 1;
                                }
                                res?
                            }
                            MediaFrame::Script {
                                runtime_stat,
                                pts,
                                payload,
                                on_meta_data: _,
                            } => {
                                runtime_stat.play_time_ns = get_timestamp_ns().unwrap_or(0);

                                self.chunk_stream.write_tag(message).await?;
                                //TODO -
                                handle.stat.script_frame_cnt += 1;
                            }
                        }
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
                tracing::error!("protocol control message should not be here: {:?}", request);
                panic!("got unexpected protocol control meessage");
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
                tracing::error!("got unexpected s2c command: {:?}", command);
            }
            RtmpUserMessageBody::SharedObject() => {
                tracing::warn!("ignore shared object command");
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
                let media_frames = Self::chunked_rtmp_frame_to_media_frame(
                    &header,
                    RtmpUserMessageBody::Audio { payload: audio },
                )?;
                for media_frame in media_frames {
                    let res = handle
                        .stream_data_producer
                        .send(media_frame)
                        .await
                        .map_err(|err| {
                            tracing::error!("send audio data to stream center failed: {:?}", err);
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

    async fn process_video(
        &mut self,
        header: ChunkMessageCommonHeader,
        video: BytesMut,
    ) -> RtmpServerResult<()> {
        match &mut self.runtime_handle {
            SessionRuntime::Publish(handle) => {
                handle.no_data_since = None;
                let media_frames = Self::chunked_rtmp_frame_to_media_frame(
                    &header,
                    RtmpUserMessageBody::Video { payload: video },
                )?;
                for media_frame in media_frames {
                    let res = handle
                        .stream_data_producer
                        .send(media_frame)
                        .await
                        .map_err(|err| {
                            tracing::error!("send video to stream center failed: {:?}", err);
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

    async fn process_meta(
        &mut self,
        header: ChunkMessageCommonHeader,
        payload: BytesMut,
    ) -> RtmpServerResult<()> {
        match &mut self.runtime_handle {
            SessionRuntime::Publish(handle) => {
                handle.no_data_since = None;
                let media_frames = Self::chunked_rtmp_frame_to_media_frame(
                    &header,
                    RtmpUserMessageBody::MetaData { payload },
                )?;

                for media_frame in media_frames {
                    let res = handle
                        .stream_data_producer
                        .send(media_frame)
                        .await
                        .map_err(|err| {
                            tracing::error!("send meta to stream center failed: {:?}", err);
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
            }
            _ => {
                tracing::error!(
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
                let media_frames = Self::chunked_rtmp_frame_to_media_frame(
                    &header,
                    RtmpUserMessageBody::Aggregate { payload: aggregate },
                )?;
                for media_frame in media_frames {
                    let res = handle
                        .stream_data_producer
                        .send(media_frame)
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

    fn chunked_rtmp_frame_to_media_frame(
        header: &ChunkMessageCommonHeader,
        frame: RtmpUserMessageBody,
    ) -> RtmpServerResult<Vec<MediaFrame>> {
        let mut runtime_stat = MediaMessageRuntimeStat {
            read_time_ns: header.runtime_stat.read_time_ns,
            session_process_time_ns: header.runtime_stat.process_time_ns,
            publish_stream_source_time_ns: get_timestamp_ns().unwrap_or(0),
            ..Default::default()
        };

        match frame {
            RtmpUserMessageBody::Audio { mut payload } => {
                runtime_stat.stream_source_received_time_ns = get_timestamp_ns().unwrap_or(0);

                let mut cursor = Cursor::new(&mut payload);
                let tag_header =
                    flv::tag::audio_tag_header::AudioTagHeader::read_from(&mut cursor)?;

                runtime_stat.stream_source_parse_time_ns = get_timestamp_ns().unwrap_or(0);

                Ok(vec![MediaFrame::Audio {
                    runtime_stat,
                    pts: header.timestamp as u64,
                    header: tag_header.try_into()?,
                    payload,
                }])
            }
            RtmpUserMessageBody::Video { mut payload } => {
                runtime_stat.stream_source_received_time_ns = get_timestamp_ns().unwrap_or(0);

                let mut cursor = Cursor::new(&mut payload);
                let tag_header =
                    flv::tag::video_tag_header::VideoTagHeader::read_from(&mut cursor)?;

                runtime_stat.stream_source_parse_time_ns = get_timestamp_ns().unwrap_or(0);

                Ok(vec![MediaFrame::Video {
                    runtime_stat,
                    pts: header.timestamp as u64,
                    header: tag_header.try_into()?,
                    payload,
                }])
            }
            RtmpUserMessageBody::MetaData { ref payload } => {
                runtime_stat.stream_source_received_time_ns = get_timestamp_ns().unwrap_or(0);
                runtime_stat.stream_source_parse_time_ns = get_timestamp_ns().unwrap_or(0);
                let mut cursor = Cursor::new(payload);
                let on_meta_data: Option<OnMetaData> =
                    OnMetaData::read_from(&mut cursor, amf::Version::Amf0);

                tracing::trace!("got script tag, onMetaData: {:?}", on_meta_data);

                Ok(vec![MediaFrame::Script {
                    runtime_stat,
                    pts: header.timestamp as u64,
                    payload: payload.clone(),
                    on_meta_data: Box::new(on_meta_data),
                }])
            }
            RtmpUserMessageBody::Aggregate { ref payload } => {
                let aggregate_chunks = Self::parse_aggregate_frame(&header, &payload)?;
                let mut res = vec![];
                for chunk in aggregate_chunks {
                    res.extend(Self::chunked_rtmp_frame_to_media_frame(header, chunk)?);
                }
                Ok(res)
            }
            _ => {
                todo!()
            }
        }
    }

    pub fn parse_aggregate_frame(
        header: &ChunkMessageCommonHeader,
        payload: &BytesMut,
    ) -> RtmpServerResult<Vec<RtmpUserMessageBody>> {
        let mut cursor = Cursor::new(payload);
        let mut timestamp_delta = None;
        let mut result = vec![];
        while cursor.has_remaining() {
            let flv_tag_header = flv::tag::FLVTag::read_tag_header_from(&mut cursor)?;
            let mut body_bytes = BytesMut::with_capacity(flv_tag_header.data_size as usize);

            body_bytes.resize(flv_tag_header.data_size as usize, 0);
            cursor.read_exact(&mut body_bytes)?;

            // skip prev tag size
            cursor.advance(4);

            if timestamp_delta.is_none() {
                timestamp_delta = Some(header.timestamp as u64 - (flv_tag_header.timestamp as u64));
            }

            match flv_tag_header.tag_type {
                FLVTagType::Audio => {
                    result.push(RtmpUserMessageBody::Audio {
                        payload: body_bytes,
                    });
                }
                FLVTagType::Video => {
                    result.push(RtmpUserMessageBody::Video {
                        payload: body_bytes,
                    });
                }
                FLVTagType::Script => {
                    result.push(RtmpUserMessageBody::MetaData {
                        payload: body_bytes,
                    });
                }
            }
        }
        Ok(result)
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
        self.chunk_stream
            .chunk_writer()
            .write_window_ack_size_message(4096)?;
        self.chunk_stream
            .chunk_writer()
            .write_set_peer_bandwidth(4096, SetPeerBandWidthLimitType::Dynamic)?;
        self.chunk_stream.flush_chunk().await?;

        self.stream_properties.app = request.command_object.app.clone();

        self.connect_info = request.command_object;

        self.chunk_stream.chunk_writer().write_connect_response(
            true,
            request.transaction_id.into(),
            super::consts::FMSVER,
            super::consts::FMS_CAPABILITIES,
            super::consts::response_code::NET_CONNECTION_CONNECT_SUCCESS,
            super::consts::response_level::STATUS,
            "Connection Succeeded.",
            self.connect_info.object_encoding,
        )?;
        self.chunk_stream.flush_chunk().await?;

        tracing::info!("connect done, connect_info: {:?}", self.connect_info,);

        Ok(())
    }

    async fn process_create_stream_command(
        &mut self,
        request: CreateStreamCommandRequest,
    ) -> RtmpServerResult<()> {
        self.chunk_stream
            .chunk_writer()
            .write_create_stream_response(
                true,
                request.transaction_id,
                None,
                RESPONSE_STREAM_ID.into(),
            )?;
        self.chunk_stream.flush_chunk().await?;
        Ok(())
    }

    async fn process_publish_command(&mut self, request: PublishCommand) -> RtmpServerResult<()> {
        let stream_type: StreamType = request.publishing_type.try_into()?;
        self.publish_to_stream_center(&request.publishing_name, stream_type)
            .await?;

        self.chunk_stream.chunk_writer().write_on_status_response(
            response_level::STATUS,
            response_code::NET_STREAM_PUBLISH_START_SUCCESS,
            "publish start",
            self.connect_info.object_encoding,
            None,
        )?;
        self.chunk_stream.flush_chunk().await?;

        tracing::info!("process publish command success");
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
        self.chunk_stream.chunk_writer().write_on_status_response(
            response_level::STATUS,
            response_code::NET_CONNECTION_CONNECT_RECONNECT_REQUEST,
            description.unwrap_or("The streaming server is undergoing updates."),
            self.connect_info.object_encoding,
            Some(tc_url_arg),
        )?;
        self.chunk_stream.flush_chunk().await?;
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
                tracing::error!(
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
        tracing::info!("process delete stream command, request: {:?}", request);
        let _ = self
            .unpublish_from_stream_center(
                &self.stream_properties.stream_name.clone(),
                &self.stream_properties.app.clone(),
            )
            .await;

        self.chunk_stream.chunk_writer().write_on_status_response(
            response_level::STATUS,
            response_code::NET_STREAM_DELETE_SUCCESS,
            "delete stream success",
            self.connect_info.object_encoding,
            None,
        )?;

        self.chunk_stream.flush_chunk().await?;

        tracing::info!("process delete stream command success");
        Ok(())
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
                tracing::error!(
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
                self.runtime_handle = SessionRuntime::Publish(PublishHandle {
                    stream_data_producer: sender,
                    no_data_since: None,
                    stat: Default::default(),
                });
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
        tracing::info!("process call request: {:?}", request);
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
                        tracing::warn!(
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

                        self.chunk_stream.chunk_writer().write_call_response(
                            res.is_ok(),
                            request.transaction_id,
                            None,
                            None,
                        )?;
                        self.chunk_stream.flush_chunk().await?;
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
                tracing::error!(
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
                tracing::error!(
                    "channel closed while trying receive subscribe result, stream_name: {}, app: {}",
                    stream_name,
                    app
                );
                Err(RtmpServerError::ChannelSendFailed {
                    backtrace: Backtrace::capture(),
                })
            }
            Ok(Err(err)) => {
                tracing::error!(
                    "subscribe from stream center failed, {:?}, stream_name: {}, app: {}",
                    err,
                    stream_name,
                    app
                );
                Err(err.into())
            }
            Ok(Ok(response)) => {
                tracing::info!(
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
                tracing::error!(
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

        self.chunk_stream
            .chunk_writer()
            .write_set_chunk_size(self.config.chunk_size)?;
        self.chunk_stream.flush_chunk().await?;
        if self.stream_properties.stream_type == StreamType::Record {
            self.chunk_stream
                .chunk_writer()
                .write_stream_ids_recorded(header.message_stream_id)?; // I bet the stream_id is useless
            self.chunk_stream.flush_chunk().await?;
        }

        self.chunk_stream
            .chunk_writer()
            .write_stream_begin(header.message_stream_id)?;
        self.chunk_stream.flush_chunk().await?;
        match subscribe_result {
            Err(err) => {
                tracing::error!("subscribe stream failed: {:?}", err);
                self.chunk_stream.chunk_writer().write_on_status_response(
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
                    self.chunk_stream.chunk_writer().write_on_status_response(
                        response_level::STATUS,
                        response_code::NET_STREAM_PLAY_RESET,
                        "reset stream",
                        self.connect_info.object_encoding,
                        None,
                    )?;
                }
                self.chunk_stream.chunk_writer().write_on_status_response(
                    response_level::STATUS,
                    response_code::NET_STREAM_PLAY_START,
                    "play start",
                    self.connect_info.object_encoding,
                    None,
                )?;
            }
        }

        self.chunk_stream.flush_chunk().await?;
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
                tracing::warn!(
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
                tracing::warn!(
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
                    tracing::warn!(
                        "got unexpected set_buffer_length event while not in a play session, event: {:?}, ignore",
                        request
                    );
                }
            },
            UserControlEvent::PingRequest { timestamp } => {
                tracing::trace!("got a ping request: {}", timestamp);
                self.chunk_stream
                    .chunk_writer()
                    .write_ping_response(timestamp)?;
                self.chunk_stream.flush_chunk().await?;
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
