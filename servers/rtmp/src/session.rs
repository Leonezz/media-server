use std::{
    backtrace::Backtrace,
    collections::HashMap,
    io::{self, Cursor, Read},
    sync::Arc,
    time::SystemTime,
};

use ::stream_center::{
    events::StreamCenterEvent,
    stream_source::{StreamIdentifier, StreamType},
};
use codec_common::video::VideoConfig;
use flv_formats::tag::{
    FLVTag,
    flv_tag_body::FLVTagBodyWithFilter,
    flv_tag_header::{FLVTagHeader, FLVTagType},
};
use num::ToPrimitive;
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
use stream_center::{events::SubscribeResponse, gop::MediaFrame};
use tokio::{
    net::TcpStream,
    sync::{
        RwLock,
        mpsc::{self},
        oneshot,
    },
};
use tokio_util::{
    bytes::{Buf, Bytes},
    either::Either,
};
use url::Url;
use utils::{
    system::time::get_timestamp_ns,
    traits::reader::{ReadFrom, ReadRemainingFrom},
};
use uuid::Uuid;

use crate::{chunk_stream::RtmpChunkStream, errors::RtmpServerError};

use super::{
    config::RtmpSessionConfig,
    consts::{response_code, response_level},
    errors::RtmpServerResult,
};

#[derive(Debug, Default, Clone)]
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

#[derive(Debug, Clone)]
struct PublishHandle {
    stream_data_producer: mpsc::Sender<MediaFrame>,
    no_data_since: Option<SystemTime>,
    stat: SessionStat,
}

#[derive(Debug)]
enum SessionRuntime {
    Play(Arc<RwLock<PlayHandle>>),
    Publish(Arc<RwLock<PublishHandle>>),
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
    video_nalu_size_length: Option<u8>,
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
            video_nalu_size_length: None,
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
                                .duration_since(
                                    handle.read().await.no_data_since.unwrap_or(current_time),
                                )
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
                let handle = handle.read().await;
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
                    for message in &messages {
                        if let MediaFrame::VideoConfig {
                            timestamp_nano: _,
                            config,
                        } = message
                        {
                            match config.as_ref() {
                                VideoConfig::H264 {
                                    sps: _,
                                    pps: _,
                                    sps_ext: _,
                                    avc_decoder_configuration_record,
                                } => {
                                    self.video_nalu_size_length = avc_decoder_configuration_record
                                        .as_ref()
                                        .map(|v| v.length_size_minus_one.checked_add(1).unwrap());
                                }
                            }
                        }
                        let tag = message.to_flv_tag(self.video_nalu_size_length.unwrap_or(4))?;
                        self.chunk_stream.write_tag(tag).await?;
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
            RtmpUserMessageBody::MetaData { payload } => match &self.runtime_handle {
                SessionRuntime::Publish(publish_handle) => {
                    self.process_meta(publish_handle.clone(), header, payload)
                        .await?
                }
                _ => {
                    unreachable!()
                }
            },
            RtmpUserMessageBody::Aggregate { payload } => match &self.runtime_handle {
                SessionRuntime::Publish(publish_handle) => {
                    self.process_aggregate(publish_handle.clone(), header, payload)
                        .await?
                }
                _ => {
                    unreachable!()
                }
            },
            RtmpUserMessageBody::Audio { payload } => match &self.runtime_handle {
                SessionRuntime::Publish(publish_handle) => {
                    self.process_audio(publish_handle.clone(), header, payload)
                        .await?
                }
                _ => {
                    unreachable!()
                }
            },
            RtmpUserMessageBody::Video { payload } => match &self.runtime_handle {
                SessionRuntime::Publish(publish_handle) => {
                    self.process_video(publish_handle.clone(), header, payload)
                        .await?
                }
                _ => {
                    unreachable!()
                }
            },
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
        publish_handle: Arc<RwLock<PublishHandle>>,
        header: ChunkMessageCommonHeader,
        audio: Bytes,
    ) -> RtmpServerResult<()> {
        let mut handle = publish_handle.write().await;
        handle.no_data_since = None;
        let media_frames = self.chunked_rtmp_frame_to_media_frame(
            &header,
            RtmpUserMessageBody::Audio { payload: audio },
            None,
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
        Ok(())
    }

    async fn process_video(
        &mut self,
        publish_handle: Arc<RwLock<PublishHandle>>,
        header: ChunkMessageCommonHeader,
        video: Bytes,
    ) -> RtmpServerResult<()> {
        let mut handle = publish_handle.write().await;
        handle.no_data_since = None;
        let media_frames = self.chunked_rtmp_frame_to_media_frame(
            &header,
            RtmpUserMessageBody::Video { payload: video },
            None,
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

        Ok(())
    }

    async fn process_meta(
        &mut self,
        publish_handle: Arc<RwLock<PublishHandle>>,
        header: ChunkMessageCommonHeader,
        payload: Bytes,
    ) -> RtmpServerResult<()> {
        let mut handle = publish_handle.write().await;
        handle.no_data_since = None;
        let media_frames = self.chunked_rtmp_frame_to_media_frame(
            &header,
            RtmpUserMessageBody::MetaData { payload },
            None,
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

        Ok(())
    }

    async fn process_aggregate(
        &mut self,
        publish_handle: Arc<RwLock<PublishHandle>>,
        header: ChunkMessageCommonHeader,
        aggregate: Bytes,
    ) -> RtmpServerResult<()> {
        let mut handle = publish_handle.write().await;
        handle.no_data_since = None;
        let media_frames = self.chunked_rtmp_frame_to_media_frame(
            &header,
            RtmpUserMessageBody::Aggregate { payload: aggregate },
            None,
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

        Ok(())
    }

    fn chunked_rtmp_frame_to_media_frame(
        &mut self,
        header: &ChunkMessageCommonHeader,
        frame: RtmpUserMessageBody,
        timestamp_delta_nano: Option<u64>,
    ) -> RtmpServerResult<Vec<MediaFrame>> {
        let flv_tags = match frame {
            RtmpUserMessageBody::Audio { payload } => {
                let flv_tag_header = FLVTagHeader {
                    tag_type: FLVTagType::Audio,
                    data_size: payload.len().to_u32().unwrap(),
                    timestamp: header
                        .timestamp
                        .checked_add(
                            timestamp_delta_nano
                                .unwrap_or(0)
                                .checked_div(1_000_000)
                                .and_then(|v| v.to_u32())
                                .unwrap(),
                        )
                        .unwrap(),
                    filter_enabled: false,
                };
                vec![(flv_tag_header, payload)]
            }
            RtmpUserMessageBody::Video { payload } => {
                let flv_tag_header = FLVTagHeader {
                    tag_type: FLVTagType::Video,
                    data_size: payload.len().to_u32().unwrap(),
                    timestamp: header
                        .timestamp
                        .checked_add(
                            timestamp_delta_nano
                                .unwrap_or(0)
                                .checked_div(1_000_000)
                                .and_then(|v| v.to_u32())
                                .unwrap(),
                        )
                        .unwrap(),
                    filter_enabled: false,
                };
                vec![(flv_tag_header, payload)]
            }

            RtmpUserMessageBody::MetaData { payload } => {
                let flv_tag_header = FLVTagHeader {
                    tag_type: FLVTagType::Script,
                    data_size: payload.len().to_u32().unwrap(),
                    timestamp: header
                        .timestamp
                        .checked_add(
                            timestamp_delta_nano
                                .unwrap_or(0)
                                .checked_div(1_000_000)
                                .and_then(|v| v.to_u32())
                                .unwrap(),
                        )
                        .unwrap(),
                    filter_enabled: false,
                };
                vec![(flv_tag_header, payload)]
            }
            RtmpUserMessageBody::Aggregate { ref payload } => {
                return self.parse_aggregate_frame(header, payload);
            }
            _ => {
                todo!()
            }
        };
        let mut result = vec![];
        for (tag_header, payload) in flv_tags {
            let flv_tag_body =
                FLVTagBodyWithFilter::read_remaining_from(&tag_header, &mut payload.reader())?;
            let flv_tag = FLVTag {
                tag_header,
                body_with_filter: flv_tag_body,
            };
            result.push(MediaFrame::from_flv_tag(
                flv_tag,
                self.video_nalu_size_length.unwrap_or(4),
            )?);
        }
        for item in &result {
            if let MediaFrame::VideoConfig {
                timestamp_nano: _,
                config,
            } = item
            {
                match config.as_ref() {
                    VideoConfig::H264 {
                        sps: _,
                        pps: _,
                        sps_ext: _,
                        avc_decoder_configuration_record,
                    } => {
                        if let Some(record) = avc_decoder_configuration_record {
                            self.video_nalu_size_length =
                                Some(record.length_size_minus_one.checked_add(1).unwrap());
                        } else {
                            unimplemented!()
                        }
                    }
                }
            }
        }
        Ok(result)
    }

    pub fn parse_aggregate_frame(
        &mut self,
        header: &ChunkMessageCommonHeader,
        payload: &Bytes,
    ) -> RtmpServerResult<Vec<MediaFrame>> {
        let mut cursor = Cursor::new(payload);
        let mut timestamp_delta_nano = None;
        let mut result = vec![];
        while cursor.has_remaining() {
            let flv_tag_header = FLVTagHeader::read_from(cursor.by_ref())?;
            let mut body_bytes = vec![0; flv_tag_header.data_size.to_usize().unwrap()];
            cursor.read_exact(&mut body_bytes)?;
            // skip prev tag size
            cursor.advance(4);
            if timestamp_delta_nano.is_none() {
                timestamp_delta_nano = Some(
                    header
                        .timestamp
                        .checked_mul(1_000_000)
                        .unwrap()
                        .to_u64()
                        .unwrap()
                        .checked_sub(
                            flv_tag_header
                                .timestamp
                                .checked_mul(1_000_000)
                                .unwrap()
                                .to_u64()
                                .unwrap(),
                        )
                        .unwrap(),
                );
            }

            let rtmp_message = match flv_tag_header.tag_type {
                FLVTagType::Audio => RtmpUserMessageBody::Audio {
                    payload: body_bytes.into(),
                },
                FLVTagType::Video => RtmpUserMessageBody::Video {
                    payload: body_bytes.into(),
                },
                FLVTagType::Script => RtmpUserMessageBody::MetaData {
                    payload: body_bytes.into(),
                },
            };

            let frames =
                self.chunked_rtmp_frame_to_media_frame(header, rtmp_message, timestamp_delta_nano)?;
            assert!(frames.len() == 1);
            result.extend(frames);
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
            amf_formats::string(new_tc_url, self.connect_info.object_encoding),
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
                self.runtime_handle =
                    SessionRuntime::Publish(Arc::new(RwLock::new(PublishHandle {
                        stream_data_producer: sender,
                        no_data_since: None,
                        stat: Default::default(),
                    })));
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
