use std::{
    cmp::{max, min},
    collections::HashMap,
    fmt::Display,
    io::{Cursor, Read},
    sync::Arc,
};

use flv::tag::{FLVTag, FLVTagType, on_meta_data::OnMetaData};
use tokio::sync::{RwLock, mpsc};
use tokio_util::bytes::{Buf, BytesMut};
use utils::system::time::get_timestamp_ns;
use uuid::Uuid;

use crate::{
    errors::{StreamCenterError, StreamCenterResult},
    frame_info::{
        AggregateMeta, AudioMeta, ChunkFrameData, MediaMessageRuntimeStat, ScriptMeta, VideoMeta,
    },
    gop::{FLVMediaFrame, GopQueue},
    signal::StreamSignal,
    stream_center::StreamSourceDynamicInfo,
};

#[derive(Debug, PartialEq, Eq)]
enum StreamStatus {
    NotStarted,
    Running,
    Stopped,
}

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, Default)]
pub enum StreamType {
    #[default]
    Live,
    Record,
    Append,
}

impl Display for StreamType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Live => f.write_str("live"),
            Self::Record => f.write_str("record"),
            Self::Append => f.write_str("append"),
        }
    }
}

impl From<StreamType> for String {
    fn from(value: StreamType) -> Self {
        match value {
            StreamType::Live => "live".to_owned(),
            StreamType::Append => "append".to_owned(),
            StreamType::Record => "record".to_owned(),
        }
    }
}

impl TryFrom<String> for StreamType {
    type Error = StreamCenterError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "live" => Ok(StreamType::Live),
            "recorded" => Ok(StreamType::Record),
            "append" => Ok(StreamType::Append),
            _ => Err(StreamCenterError::InvalidStreamType(value)),
        }
    }
}

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct StreamIdentifier {
    pub stream_name: String,
    pub app: String,
}

#[derive(Debug)]
pub enum ConsumeGopCache {
    None,
    All,
    GopCount(u64),
}

#[derive(Debug, Default)]
pub struct PlayStat {
    video_sh_sent: bool,
    audio_sh_sent: bool,
    first_key_frame_sent: bool,
    video_frames_sent: u64,
    audio_frames_sent: u64,
    script_frames_sent: u64,

    video_frame_send_fail_cnt: u64,
    audio_frame_send_fail_cnt: u64,
    script_frame_send_fail_cnt: u64,
}

#[derive(Debug)]
pub struct SubscribeHandler {
    pub context: HashMap<String, String>,
    pub parsed_context: ParsedContext,
    pub data_sender: mpsc::Sender<FLVMediaFrame>,
    pub stat: PlayStat,
}

#[derive(Debug)]
pub struct ParsedContext {
    // videoOnly
    pub video_only: bool,
    // audioOnly
    pub audio_only: bool,
    // backtrackGopCnt
    pub backtrack_gop_cnt: ConsumeGopCache,
}

impl From<&HashMap<String, String>> for ParsedContext {
    fn from(value: &HashMap<String, String>) -> Self {
        Self {
            video_only: value.contains_key("videoOnly"),
            audio_only: value.contains_key("audioOnly"),
            backtrack_gop_cnt: value.get("backtraceGopCnt").map_or_else(
                || ConsumeGopCache::GopCount(1),
                |s| ConsumeGopCache::GopCount(s.parse().unwrap_or(0)),
            ),
        }
    }
}

#[derive(Debug)]
pub struct StreamSource {
    pub identifier: StreamIdentifier,
    pub stream_type: StreamType,

    data_receiver: mpsc::Receiver<ChunkFrameData>,
    data_distributer: Arc<RwLock<HashMap<Uuid, SubscribeHandler>>>,
    stream_dynamic_info: Arc<RwLock<StreamSourceDynamicInfo>>,
    // data_consumer: broadcast::Receiver<FrameData>,
    status: StreamStatus,
    signal_receiver: mpsc::Receiver<StreamSignal>,
    gop_cache: GopQueue,
}

impl StreamSource {
    pub fn new(
        stream_name: &str,
        app: &str,
        stream_type: StreamType,
        data_receiver: mpsc::Receiver<ChunkFrameData>,
        signal_receiver: mpsc::Receiver<StreamSignal>,
        data_distributer: Arc<RwLock<HashMap<Uuid, SubscribeHandler>>>,
        stream_dynamic_info: Arc<RwLock<StreamSourceDynamicInfo>>,
    ) -> Self {
        Self {
            identifier: StreamIdentifier {
                stream_name: stream_name.to_string(),
                app: app.to_string(),
            },
            stream_type,
            data_receiver,
            data_distributer,
            stream_dynamic_info,
            // data_consumer: rx,
            gop_cache: GopQueue::new(600_000, 100_000),
            status: StreamStatus::NotStarted,
            signal_receiver,
        }
    }

    pub async fn run(&mut self) -> StreamCenterResult<()> {
        if self.status == StreamStatus::Running {
            return Ok(());
        }
        self.status = StreamStatus::Running;
        log::info!("stream is running, stream id: {:?}", self.identifier);

        loop {
            match self.data_receiver.recv().await {
                None => {}
                Some(data) => {
                    let flv_frames = self.chunked_frame_to_flv_frame(data)?;
                    for flv_frame in flv_frames {
                        self.on_flv_frame(flv_frame).await?;
                    }
                }
            }
            match self.signal_receiver.try_recv() {
                Err(_) => {}
                Ok(signal) => match signal {
                    StreamSignal::Stop => {
                        self.status = StreamStatus::Stopped;
                        return Ok(());
                    }
                },
            }
        }
    }
    fn parse_aggregate_frame(
        &mut self,
        aggregate_meta: &AggregateMeta,
        payload: &BytesMut,
    ) -> StreamCenterResult<Vec<ChunkFrameData>> {
        let mut cursor = Cursor::new(payload);
        let mut timestamp_delta = None;
        let mut result = vec![];
        while cursor.has_remaining() {
            let flv_tag_header = FLVTag::read_tag_header_from(&mut cursor)?;
            let mut body_bytes = BytesMut::with_capacity(flv_tag_header.data_size as usize);

            body_bytes.resize(flv_tag_header.data_size as usize, 0);
            cursor.read_exact(&mut body_bytes)?;

            // skip prev tag size
            cursor.advance(4);

            if timestamp_delta.is_none() {
                timestamp_delta = Some(aggregate_meta.pts - (flv_tag_header.timestamp as u64));
            }

            match flv_tag_header.tag_type {
                FLVTagType::Audio => {
                    result.push(ChunkFrameData::Audio {
                        meta: AudioMeta {
                            pts: aggregate_meta.pts + timestamp_delta.expect("this cannot be none"),
                            runtime_stat: MediaMessageRuntimeStat {
                                read_time_ns: aggregate_meta.read_time_ns,
                                session_process_time_ns: aggregate_meta.session_process_time_ns,
                                publish_stream_source_time_ns: aggregate_meta
                                    .publish_stream_source_time_ns,
                                ..Default::default()
                            },
                        },
                        payload: body_bytes,
                    });
                }
                FLVTagType::Video => {
                    result.push(ChunkFrameData::Video {
                        meta: VideoMeta {
                            pts: aggregate_meta.pts + timestamp_delta.expect("this cannot be none"),
                            runtime_stat: MediaMessageRuntimeStat {
                                read_time_ns: aggregate_meta.read_time_ns,
                                session_process_time_ns: aggregate_meta.session_process_time_ns,
                                publish_stream_source_time_ns: aggregate_meta
                                    .publish_stream_source_time_ns,
                                stream_source_received_time_ns: get_timestamp_ns().unwrap_or(0),
                                ..Default::default()
                            },
                        },
                        payload: body_bytes,
                    });
                }
                FLVTagType::Script => {
                    result.push(ChunkFrameData::Script {
                        meta: ScriptMeta {
                            pts: aggregate_meta.pts + timestamp_delta.expect("this cannot be none"),
                            runtime_stat: MediaMessageRuntimeStat {
                                read_time_ns: aggregate_meta.read_time_ns,
                                session_process_time_ns: aggregate_meta.session_process_time_ns,
                                publish_stream_source_time_ns: aggregate_meta
                                    .publish_stream_source_time_ns,
                                stream_source_received_time_ns: get_timestamp_ns().unwrap_or(0),
                                ..Default::default()
                            },
                        },
                        payload: body_bytes,
                    });
                }
            }
        }
        Ok(result)
    }

    fn chunked_frame_to_flv_frame(
        &mut self,
        frame: ChunkFrameData,
    ) -> StreamCenterResult<Vec<FLVMediaFrame>> {
        match frame {
            ChunkFrameData::Audio {
                mut meta,
                mut payload,
            } => {
                meta.runtime_stat.stream_source_received_time_ns = get_timestamp_ns().unwrap_or(0);

                let mut cursor = Cursor::new(&mut payload);
                let tag_header =
                    flv::tag::audio_tag_header::AudioTagHeader::read_from(&mut cursor)?;

                meta.runtime_stat.stream_source_parse_time_ns = get_timestamp_ns().unwrap_or(0);

                Ok(vec![FLVMediaFrame::Audio {
                    runtime_stat: meta.runtime_stat,
                    pts: meta.pts,
                    header: tag_header.try_into()?,
                    payload,
                }])
            }
            ChunkFrameData::Video {
                mut meta,
                mut payload,
            } => {
                meta.runtime_stat.stream_source_received_time_ns = get_timestamp_ns().unwrap_or(0);

                let mut cursor = Cursor::new(&mut payload);
                let tag_header =
                    flv::tag::video_tag_header::VideoTagHeader::read_from(&mut cursor)?;

                meta.runtime_stat.stream_source_parse_time_ns = get_timestamp_ns().unwrap_or(0);

                Ok(vec![FLVMediaFrame::Video {
                    runtime_stat: meta.runtime_stat,
                    pts: meta.pts,
                    header: tag_header.try_into()?,
                    payload,
                }])
            }
            ChunkFrameData::Script {
                mut meta,
                ref payload,
            } => {
                meta.runtime_stat.stream_source_received_time_ns = get_timestamp_ns().unwrap_or(0);
                meta.runtime_stat.stream_source_parse_time_ns = get_timestamp_ns().unwrap_or(0);
                let mut cursor = Cursor::new(payload);
                let on_meta_data: Option<OnMetaData> =
                    OnMetaData::read_from(&mut cursor, amf::Version::Amf0);

                log::trace!("got script tag, onMetaData: {:?}", on_meta_data);

                Ok(vec![FLVMediaFrame::Script {
                    runtime_stat: meta.runtime_stat,
                    pts: meta.pts,
                    payload: payload.clone(),
                    on_meta_data: Box::new(on_meta_data),
                }])
            }
            ChunkFrameData::Aggregate { meta, data } => {
                let aggregate_chunks = self.parse_aggregate_frame(&meta, &data)?;
                let mut result = vec![];
                for chunk in aggregate_chunks {
                    result.extend(self.chunked_frame_to_flv_frame(chunk)?);
                }
                Ok(result)
            }
        }
    }

    async fn on_flv_frame(&mut self, frame: FLVMediaFrame) -> StreamCenterResult<()> {
        if let Err(err) = self.gop_cache.append_frame(frame.clone()) {
            log::error!("append frame to gop cache failed: {:?}", err);
        }

        if self.data_distributer.read().await.is_empty() {
            return Ok(());
        }

        let update_stat = |stat: &mut PlayStat, frame: &FLVMediaFrame, fail: bool| {
            if frame.is_video() {
                stat.video_frame_send_fail_cnt += <bool as Into<u64>>::into(fail);
                stat.video_frames_sent += <bool as Into<u64>>::into(!fail);
            } else if frame.is_audio() {
                stat.audio_frame_send_fail_cnt += <bool as Into<u64>>::into(fail);
                stat.audio_frames_sent += <bool as Into<u64>>::into(!fail);
            } else {
                stat.script_frame_send_fail_cnt += <bool as Into<u64>>::into(fail);
                stat.script_frames_sent += <bool as Into<u64>>::into(!fail);
            }
        };

        for (key, handler) in &mut self.data_distributer.write().await.iter_mut() {
            if (!handler.stat.audio_sh_sent && !handler.parsed_context.video_only)
                || (!handler.stat.video_sh_sent && !handler.parsed_context.audio_only)
            {
                self.on_new_consumer(key, handler, update_stat).await?;
            }
            if handler.parsed_context.audio_only && frame.is_video() {
                continue;
            }
            if handler.parsed_context.video_only && frame.is_audio() {
                continue;
            }
            let res = handler.data_sender.try_send(frame.clone());
            if res.is_err() {
                log::error!("distribute frame data to {} failed: {:?}", key, res);
            }
            if frame.is_video_key_frame() {
                handler.stat.first_key_frame_sent = true;
            }
            update_stat(&mut handler.stat, &frame, res.is_err());
        }

        Ok(())
    }

    async fn on_new_consumer<F>(
        &self,
        key: &Uuid,
        handler: &mut SubscribeHandler,
        update_stat: F,
    ) -> StreamCenterResult<()>
    where
        F: Fn(&mut PlayStat, &FLVMediaFrame, bool),
    {
        // we trust the gop stats after 3 gops (but why?)
        if self.gop_cache.gops.len() > 2 {
            self.stream_dynamic_info.write().await.has_audio =
                self.gop_cache.get_audio_frame_cut() > 0;
            self.stream_dynamic_info.write().await.has_video =
                self.gop_cache.get_video_frame_cnt() > 0;
        }

        if let Some(script) = &self.gop_cache.script_frame {
            let res = handler.data_sender.try_send(script.clone());
            if res.is_err() {
                log::error!("distribute script frame data to {} failed: {:?}", key, res);
                handler.stat.script_frame_send_fail_cnt += 1;
            } else {
                handler.stat.script_frames_sent += 1;
            }
        }
        if let Some(video_sh) = &self.gop_cache.video_sequence_header {
            if !handler.parsed_context.audio_only {
                let res = handler.data_sender.try_send(video_sh.clone());
                if res.is_err() {
                    log::error!(
                        "distribute video sh frame data to {} failed: {:?}",
                        key,
                        res
                    );
                    handler.stat.video_frame_send_fail_cnt += 1;
                } else {
                    handler.stat.video_sh_sent = true;
                    handler.stat.video_frames_sent += 1;
                }
            }
        }
        if let Some(audio_sh) = &self.gop_cache.audio_sequence_header {
            if !handler.parsed_context.video_only {
                let res = handler.data_sender.try_send(audio_sh.clone());
                if res.is_err() {
                    log::error!(
                        "distribute audio sh frame data to {} failed: {:?}",
                        key,
                        res
                    );
                    handler.stat.audio_frame_send_fail_cnt += 1;
                } else {
                    handler.stat.audio_sh_sent = true;
                    handler.stat.audio_frames_sent += 1;
                }
            }
        }

        let total_gop_cnt = self.gop_cache.get_gops_cnt();

        if total_gop_cnt == 0 {
            log::info!("got new consumer {} but no gop cached", key);
            return Ok(());
        }

        let gop_consumer_cnt = min(
            max(
                match handler.parsed_context.backtrack_gop_cnt {
                    ConsumeGopCache::All => total_gop_cnt,
                    ConsumeGopCache::GopCount(cnt) => cnt as usize,
                    ConsumeGopCache::None => 0,
                },
                1, // always send at least one gop
            ),
            total_gop_cnt,
        );

        log::info!(
            "dump {} gops for play id: {}, total gop cnt: {}",
            gop_consumer_cnt,
            key,
            self.gop_cache.get_gops_cnt()
        );

        for index in (total_gop_cnt - gop_consumer_cnt)..total_gop_cnt {
            let gop = self.gop_cache.gops.get(index).expect("this cannot be none");

            log::info!(
                "dump gop index: {}, frame cnt: {}",
                index,
                gop.flv_frames.len()
            );
            for frame in &gop.flv_frames {
                if handler.parsed_context.audio_only && frame.is_video() {
                    continue;
                }
                if handler.parsed_context.video_only && frame.is_audio() {
                    continue;
                }
                let res = handler.data_sender.try_send(frame.clone());
                if let Err(err) = &res {
                    log::error!(
                        "distribute audio sh frame data to {} failed: {:?}",
                        key,
                        err
                    );
                }
                update_stat(&mut handler.stat, frame, res.is_err());
            }

            // there must be some key frames
            handler.stat.first_key_frame_sent = true;
        }
        Ok(())
    }
}
