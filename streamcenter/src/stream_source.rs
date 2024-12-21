use core::time;
use std::{
    backtrace::Backtrace,
    borrow::BorrowMut,
    cmp::min,
    collections::{HashMap, VecDeque},
    fmt::Display,
    hash::{self, Hash},
    io::Cursor,
    sync::Arc,
};

use flv::header;
use tokio::sync::{
    RwLock,
    broadcast::{self},
    mpsc,
};
use uuid::Uuid;

use crate::{
    errors::{StreamCenterError, StreamCenterResult},
    frame_info::FrameData,
    gop::{Gop, GopQueue},
    signal::StreamSignal,
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

impl Into<String> for StreamType {
    fn into(self) -> String {
        match self {
            Self::Live => "live".to_string(),
            Self::Record => "record".to_string(),
            Self::Append => "append".to_string(),
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
            _ => Err(StreamCenterError::InvalidStreamType(value.into())),
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
    meta_sh_sent: bool,
    video_frames_sent: u64,
    audio_frames_sent: u64,
    meta_frames_sent: u64,

    video_frame_send_fail_cnt: u64,
    audio_frame_send_fail_cnt: u64,
    meta_frame_send_fail_cnt: u64,
}

#[derive(Debug)]
pub struct SubscribeHandler {
    pub gop_cache_consume_param: ConsumeGopCache,
    pub data_sender: mpsc::Sender<FrameData>,
    pub stat: PlayStat,
}

#[derive(Debug)]
pub struct StreamSource {
    pub identifier: StreamIdentifier,
    pub stream_type: StreamType,

    context: HashMap<String, serde_json::Value>,
    data_receiver: mpsc::Receiver<FrameData>,
    data_distributer: Arc<RwLock<HashMap<Uuid, SubscribeHandler>>>,
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
        context: HashMap<String, serde_json::Value>,
        data_receiver: mpsc::Receiver<FrameData>,
        signal_receiver: mpsc::Receiver<StreamSignal>,
        data_distributer: Arc<RwLock<HashMap<Uuid, SubscribeHandler>>>,
    ) -> Self {
        Self {
            identifier: StreamIdentifier {
                stream_name: stream_name.to_string(),
                app: app.to_string(),
            },
            stream_type,
            context,
            data_receiver,
            data_distributer,
            // data_consumer: rx,
            gop_cache: GopQueue::new(100_1000, 1000),
            status: StreamStatus::NotStarted,
            signal_receiver,
        }
    }

    pub async fn run(&mut self) -> StreamCenterResult<()> {
        if self.status == StreamStatus::Running {
            return Ok(());
        }
        self.status = StreamStatus::Running;
        tracing::info!("stream is running, stream id: {:?}", self.identifier);

        loop {
            match self.data_receiver.recv().await {
                None => {}
                Some(data) => {
                    self.on_frame_data(data).await?;
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

    async fn on_frame_data(&mut self, mut frame: FrameData) -> StreamCenterResult<()> {
        match &mut frame {
            FrameData::Audio { meta, data } => {
                let mut cursor = Cursor::new(data);
                let tag_header =
                    flv::tag::audio_tag_header::AudioTagHeader::read_from(&mut cursor)?;
                meta.tag_header = tag_header;
                meta.dts =
                    utils::system::util::get_timestamp_ms().expect("this is very unlikely to fail");
                if meta.tag_header.is_aac_sequence_header() {
                    tracing::info!("got aac seq header");
                }
            }
            FrameData::Video { meta, data } => {
                let mut cursor = Cursor::new(data);
                let tag_header =
                    flv::tag::video_tag_header::VideoTagHeader::read_from(&mut cursor)?;
                meta.tag_header = tag_header;
                meta.dts =
                    utils::system::util::get_timestamp_ms().expect("this is very unlikely to fail");
                // if let Some(time) = tag_header.composition_time {
                //     meta.pts = time.into();
                // }
                if meta.tag_header.is_sequence_header() {
                    tracing::info!("got avc seq header");
                }
            }
            _ => {}
        }

        if let Err(err) = self.gop_cache.append_frame(frame.clone()) {
            tracing::error!("append frame to gop cache failed: {:?}", err);
        }

        if self.data_distributer.read().await.len() == 0 {
            return Ok(());
        }

        let update_stat = |stat: &mut PlayStat, frame: &FrameData, fail: bool| match frame {
            FrameData::Video { meta: _, data: _ } => {
                stat.video_frame_send_fail_cnt += <bool as Into<u64>>::into(fail);
                stat.video_frames_sent += <bool as Into<u64>>::into(!fail);
            }
            FrameData::Audio { meta: _, data: _ } => {
                stat.audio_frame_send_fail_cnt += <bool as Into<u64>>::into(fail);
                stat.audio_frames_sent += <bool as Into<u64>>::into(!fail);
            }
            FrameData::Meta {
                timestamp: _,
                data: _,
            } => {
                stat.meta_frame_send_fail_cnt += <bool as Into<u64>>::into(fail);
                stat.meta_frames_sent += <bool as Into<u64>>::into(!fail)
            }
            _ => {
                //TODO -
            }
        };

        for (key, handler) in &mut self.data_distributer.write().await.iter_mut() {
            if !handler.stat.audio_sh_sent || !handler.stat.video_sh_sent {
                if let Some(video_sh) = &self.gop_cache.video_sequence_header {
                    let res = handler.data_sender.try_send(video_sh.clone());
                    if res.is_err() {
                        tracing::error!(
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
                if let Some(audio_sh) = &self.gop_cache.audio_sequence_header {
                    let res = handler.data_sender.try_send(audio_sh.clone());
                    if res.is_err() {
                        tracing::error!(
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

                let total_gop_cnt = self.gop_cache.get_gops_cnt();

                let gop_consumer_cnt = match handler.gop_cache_consume_param {
                    ConsumeGopCache::All => total_gop_cnt,
                    ConsumeGopCache::GopCount(cnt) => min(total_gop_cnt, cnt as usize),
                    ConsumeGopCache::None => min(1, total_gop_cnt),
                };

                tracing::info!(
                    "dump {} gops for play id: {}, total gop cnt: {}",
                    gop_consumer_cnt,
                    key,
                    self.gop_cache.get_gops_cnt()
                );

                for index in (total_gop_cnt - gop_consumer_cnt)..total_gop_cnt {
                    let gop = self.gop_cache.gops.get(index).expect("this cannot be none");

                    tracing::info!("dump gop index: {}, frame cnt: {}", index, gop.frames.len());
                    for frame in &gop.frames {
                        let res = handler.data_sender.try_send(frame.clone());
                        if let Err(err) = &res {
                            tracing::error!(
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
            }

            let res = handler.data_sender.try_send(frame.clone());
            if res.is_err() {
                tracing::error!("distribute frame data to {} failed: {:?}", key, res);
            }
            if frame.is_video_key_frame() {
                handler.stat.first_key_frame_sent = true;
            }
            update_stat(&mut handler.stat, &frame, res.is_err());
        }

        Ok(())
    }
}
