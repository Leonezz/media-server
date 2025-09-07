use crate::{
    errors::StreamCenterResult,
    gop::{GopQueue, MediaFrame},
    make_fake_on_meta_data,
    mix_queue::MixQueue,
    signal::StreamSignal,
    stream_center::StreamSourceDynamicInfo,
};
use codec_common::{
    audio::{AudioCodecCommon, AudioConfig},
    video::VideoCodecCommon,
};
use num::ToPrimitive;
use std::{
    cmp::{max, min},
    collections::HashMap,
    sync::Arc,
    time::SystemTime,
};
use tokio::sync::{RwLock, mpsc};
use tokio_util::bytes::Bytes;
use tracing::trace_span;
use utils::traits::buffer::GenericSequencer;
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq)]
enum StreamStatus {
    NotStarted,
    Running,
    Stopped,
}

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub enum PublishProtocol {
    RTMP,
    RTSP,
}

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub enum PlayProtocol {
    RTMP,
    HTTPFLV,
    RTSP,
}

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct StreamIdentifier {
    pub stream_name: String,
    pub app: String,
}

#[derive(Debug, Clone)]
pub enum ConsumeGopCache {
    None,
    All,
    GopCount(u64),
}

#[derive(Debug, Default, Clone)]
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
    pub id: Uuid,
    pub context: HashMap<String, String>,
    pub parsed_context: ParsedContext,
    pub data_sender: mpsc::Sender<MediaFrame>,
    pub stat: PlayStat,
    pub play_protocol: PlayProtocol,
}

#[derive(Debug, Clone)]
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
    pub(crate) identifier: StreamIdentifier,
    pub(crate) publish_protocol: PublishProtocol,
    pub(crate) publish_start_time: SystemTime,

    data_receiver: mpsc::Receiver<MediaFrame>,
    data_distributer: Arc<RwLock<HashMap<Uuid, SubscribeHandler>>>,
    stream_dynamic_info: Arc<RwLock<StreamSourceDynamicInfo>>,
    // data_consumer: broadcast::Receiver<FrameData>,
    status: StreamStatus,
    signal_receiver: mpsc::Receiver<StreamSignal>,
    gop_cache: GopQueue,
    mix_queue: MixQueue,
}

impl StreamSource {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        stream_name: &str,
        app: &str,
        publish_protocol: PublishProtocol,
        data_receiver: mpsc::Receiver<MediaFrame>,
        signal_receiver: mpsc::Receiver<StreamSignal>,
        data_distributer: Arc<RwLock<HashMap<Uuid, SubscribeHandler>>>,
        stream_dynamic_info: Arc<RwLock<StreamSourceDynamicInfo>>,
    ) -> Self {
        Self {
            identifier: StreamIdentifier {
                stream_name: stream_name.to_string(),
                app: app.to_string(),
            },
            publish_protocol,
            publish_start_time: SystemTime::now(),
            data_receiver,
            data_distributer,
            stream_dynamic_info,
            // data_consumer: rx,
            gop_cache: GopQueue::new(6_0000, 8000),
            status: StreamStatus::NotStarted,
            signal_receiver,
            mix_queue: MixQueue::new(100, 100, 30),
        }
    }

    pub async fn run(&mut self) -> StreamCenterResult<()> {
        if self.status == StreamStatus::Running {
            return Ok(());
        }
        self.status = StreamStatus::Running;
        tracing::info!("stream is running, stream id: {:?}", self.identifier);
        loop {
            match tokio::time::timeout(
                tokio::time::Duration::from_millis(10),
                self.data_receiver.recv(),
            )
            .await
            {
                Err(_) => {}
                Ok(None) => {}
                Ok(Some(frame)) => {
                    if (frame.is_video() || frame.is_audio()) && !frame.is_sequence_header() {
                        let _ = self.mix_queue.enqueue(frame).inspect_err(|err| {
                            tracing::error!("enqueue frame to mix queue failed: {:?}", err);
                        });

                        for frame in self.mix_queue.try_dump() {
                            if let Err(err) = self.on_media_frame(frame).await {
                                tracing::error!("on media frame failed: {:?}", err);
                                return Err(err);
                            }
                        }
                    } else {
                        // sequence header or script frame
                        if let Err(err) = self.on_media_frame(frame).await {
                            tracing::error!("on media frame failed: {:?}", err);
                            return Err(err);
                        }
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

    async fn on_media_frame(&mut self, frame: MediaFrame) -> StreamCenterResult<()> {
        match &frame {
            MediaFrame::AudioConfig {
                timestamp_nano: _,
                sound_info: _,
                config,
            } => self.stream_dynamic_info.write().await.audio_config = Some(*config.clone()),
            MediaFrame::VideoConfig {
                timestamp_nano: _,
                config,
            } => self.stream_dynamic_info.write().await.video_config = Some(*config.clone()),
            _ => {}
        }
        if let Err(err) = self.gop_cache.append_frame(frame.clone()) {
            tracing::error!("append frame to gop cache failed: {:?}", err);
        }

        if self.data_distributer.read().await.is_empty() {
            return Ok(());
        }

        let update_stat = |stat: &mut PlayStat, frame: &MediaFrame, fail: bool| {
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

        if !self.data_distributer.read().await.is_empty() && self.gop_cache.script_frame.is_none() {
            let audio_codec = self.gop_cache.audio_config.as_ref().map(|(v, _)| match v {
                AudioConfig::AAC(_) => AudioCodecCommon::AAC,
            });
            if let Some((video_codec, video_height, video_width)) =
                self.gop_cache.video_config.as_ref().map(|v| match v {
                    codec_common::video::VideoConfig::H264 {
                        sps,
                        pps: _,
                        sps_ext: _,
                        avc_decoder_configuration_record: _,
                    } => (
                        VideoCodecCommon::AVC,
                        sps.as_ref().map_or(0, |v| v.get_video_height()),
                        sps.as_ref().map_or(0, |v| v.get_video_width()),
                    ),
                })
                && let Some(audio_codec) = audio_codec
            {
                let fake_meta = Some(make_fake_on_meta_data(
                    audio_codec,
                    video_codec,
                    video_height.to_f64().unwrap(),
                    video_width.to_f64().unwrap(),
                ));
                tracing::info!("make fake meta: {:?}", fake_meta);
                self.gop_cache.script_frame = Some(MediaFrame::Script {
                    timestamp_nano: 0,
                    on_meta_data: Box::new(fake_meta),
                    payload: Bytes::new(),
                })
            }
        }

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
                tracing::error!("distribute frame data to {} failed: {:?}", key, res);
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
        F: Fn(&mut PlayStat, &MediaFrame, bool),
    {
        let span = trace_span!(
            "new comsumer dump gop cache",
            play_id = ?key,
            gops_cnt=self.gop_cache.gops.len(),
            video_cnt=self.gop_cache.get_video_frame_cnt(),
            audio_cnt=self.gop_cache.get_audio_frame_cut()
        );
        let _enter = span.enter();
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
                tracing::error!("distribute script frame data to {} failed: {:?}", key, res);
                handler.stat.script_frame_send_fail_cnt += 1;
            } else {
                tracing::info!("distribute script frame data to {} succeed", key);
                handler.stat.script_frames_sent += 1;
            }
        }

        if let Some(video_sh) = &self.gop_cache.video_config {
            if !handler.parsed_context.audio_only {
                let res = handler.data_sender.try_send(MediaFrame::VideoConfig {
                    timestamp_nano: 0,
                    config: Box::new(video_sh.clone()),
                });
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
                    tracing::info!("distribute video sh frame to {} succeed", key);
                }
            }
        } else {
            handler.stat.video_sh_sent = true;
        }

        if let Some(audio_sh) = &self.gop_cache.audio_config {
            if !handler.parsed_context.video_only {
                let res = handler.data_sender.try_send(MediaFrame::AudioConfig {
                    timestamp_nano: 0,
                    sound_info: audio_sh.1,
                    config: Box::new(audio_sh.0.clone()),
                });
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
                    tracing::info!("distribute audio sh frame to {} succeed", key);
                }
            }
        } else {
            handler.stat.audio_sh_sent = true;
        }

        let total_gop_cnt = self.gop_cache.get_gops_cnt();

        if total_gop_cnt == 0 {
            tracing::info!("got new consumer {} but no gop cached", key);
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

        tracing::info!("dump {} gops", gop_consumer_cnt);

        for index in (total_gop_cnt - gop_consumer_cnt)..total_gop_cnt {
            let gop = self.gop_cache.gops.get(index).expect("this cannot be none");

            let span = tracing::trace_span!(
                "dump gop",
                gop_index = index,
                frame_cnt = gop.media_frames.len()
            );
            let _enter = span.enter();
            tracing::info!("start dump");
            for frame in &gop.media_frames {
                if handler.parsed_context.audio_only && frame.is_video() {
                    continue;
                }
                if handler.parsed_context.video_only && frame.is_audio() {
                    continue;
                }
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
        Ok(())
    }
}
