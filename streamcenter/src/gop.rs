use crate::errors::StreamCenterResult;
use codec_common::{
    FrameType,
    audio::{AudioCodecCommon, AudioConfig, AudioFrameInfo, SoundInfoCommon},
    video::{VideoCodecCommon, VideoConfig, VideoFrameInfo, VideoFrameUnit},
};
use flv_formats::tag::on_meta_data::OnMetaData;
use num::ToPrimitive;
use std::collections::VecDeque;
use tokio_util::bytes::Bytes;

#[derive(Debug, Clone)]
pub enum MediaFrame {
    VideoConfig {
        timestamp_nano: u64,
        config: Box<VideoConfig>,
    },
    Video {
        frame_info: VideoFrameInfo,
        payload: VideoFrameUnit,
    },
    AudioConfig {
        timestamp_nano: u64,
        sound_info: SoundInfoCommon,
        config: Box<AudioConfig>,
    },
    Audio {
        // NOTE - this tag_header is also included in the frame payload
        frame_info: AudioFrameInfo,
        payload: Bytes,
    },
    Script {
        timestamp_nano: u64,
        // onMetaData should be the content of payload,
        // note the payload still holds all the bytes
        on_meta_data: Box<Option<OnMetaData>>,
        payload: Bytes,
    },
}

impl MediaFrame {
    #[inline]
    pub fn is_video(&self) -> bool {
        matches!(
            self,
            MediaFrame::Video {
                frame_info: _,
                payload: _,
            } | MediaFrame::VideoConfig { .. }
        )
    }

    #[inline]
    pub fn get_codec_name(&self) -> &'static str {
        if self.is_audio() {
            match self.audio_codec_id() {
                Some(id) => id.get_codec_name(),
                None => "Known",
            }
        } else if self.is_video() {
            match self.video_codec_id() {
                Some(id) => id.get_codec_name(),
                None => "Known",
            }
        } else {
            "Known"
        }
    }

    #[inline]
    pub fn is_audio(&self) -> bool {
        matches!(
            self,
            MediaFrame::Audio {
                frame_info: _,
                payload: _,
            } | MediaFrame::AudioConfig { .. }
        )
    }

    #[inline]
    pub fn is_script(&self) -> bool {
        matches!(
            self,
            MediaFrame::Script {
                timestamp_nano: _,
                payload: _,
                on_meta_data: _,
            }
        )
    }

    pub fn get_presentation_timestamp_ns(&self) -> u64 {
        match self {
            Self::Audio {
                frame_info: AudioFrameInfo { timestamp_nano, .. },
                ..
            }
            | Self::AudioConfig { timestamp_nano, .. }
            | Self::VideoConfig { timestamp_nano, .. }
            | Self::Script { timestamp_nano, .. } => *timestamp_nano,
            Self::Video {
                frame_info: VideoFrameInfo { timestamp, .. },
                ..
            } => timestamp.pts(),
        }
    }

    pub fn get_presentation_timestamp_ms(&self) -> u64 {
        self.get_presentation_timestamp_ns()
            .checked_div(1_000_000)
            .unwrap()
    }

    pub fn get_decode_timestamp_ns(&self) -> u64 {
        match self {
            Self::Audio {
                frame_info: AudioFrameInfo { timestamp_nano, .. },
                ..
            }
            | Self::AudioConfig { timestamp_nano, .. }
            | Self::VideoConfig { timestamp_nano, .. }
            | Self::Script { timestamp_nano, .. } => *timestamp_nano,
            Self::Video {
                frame_info: VideoFrameInfo { timestamp, .. },
                ..
            } => timestamp.dts(),
        }
    }

    pub fn get_decode_timestamp_ms(&self) -> u64 {
        self.get_decode_timestamp_ns()
            .checked_div(1_000_000)
            .unwrap()
    }

    pub fn set_presentation_timestamp_ns(&mut self, pts_nano: u64) {
        match self {
            Self::Audio {
                frame_info: AudioFrameInfo { timestamp_nano, .. },
                ..
            }
            | Self::AudioConfig { timestamp_nano, .. }
            | Self::VideoConfig { timestamp_nano, .. }
            | Self::Script { timestamp_nano, .. } => *timestamp_nano = pts_nano,
            Self::Video {
                frame_info: VideoFrameInfo { timestamp, .. },
                ..
            } => {
                timestamp.set_pts(pts_nano);
            }
        }
    }

    pub fn set_presentation_timestamp_ms(&mut self, pts_ms: u64) {
        let ts = pts_ms.checked_mul(1_000_000).unwrap();
        self.set_presentation_timestamp_ns(ts);
    }

    pub fn set_decode_timestamp_ns(&mut self, dts_nano: u64) {
        match self {
            Self::Audio {
                frame_info: AudioFrameInfo { timestamp_nano, .. },
                ..
            }
            | Self::AudioConfig { timestamp_nano, .. }
            | Self::VideoConfig { timestamp_nano, .. }
            | Self::Script { timestamp_nano, .. } => *timestamp_nano = dts_nano,
            Self::Video {
                frame_info: VideoFrameInfo { timestamp, .. },
                ..
            } => {
                timestamp.set_dts(dts_nano);
            }
        }
    }

    pub fn set_decode_timestamp_ms(&mut self, dts_ms: u64) {
        let ts = dts_ms.checked_mul(1_000_000).unwrap();
        self.set_decode_timestamp_ns(ts);
    }

    #[inline]
    pub fn is_sequence_header(&self) -> bool {
        matches!(
            self,
            MediaFrame::AudioConfig { .. } | MediaFrame::VideoConfig { .. }
        )
    }

    #[inline]
    pub fn is_video_key_frame(&self) -> bool {
        match self {
            MediaFrame::Video {
                frame_info,
                payload: _,
            } => frame_info.frame_type == FrameType::KeyFrame,
            _ => false,
        }
    }

    pub fn video_codec_id(&self) -> Option<VideoCodecCommon> {
        match self {
            Self::Video {
                frame_info,
                payload: _,
            } => Some(frame_info.codec_id),
            _ => None,
        }
    }

    pub fn audio_codec_id(&self) -> Option<AudioCodecCommon> {
        match self {
            Self::Audio {
                frame_info,
                payload: _,
            } => Some(frame_info.codec_id),
            _ => None,
        }
    }

    pub fn same_codec_video(&self, other: &Self) -> bool {
        if !self.is_video() || !other.is_video() {
            return false;
        }
        self.video_codec_id() == other.video_codec_id()
    }
}

#[derive(Debug)]
pub struct Gop {
    pub media_frames: VecDeque<MediaFrame>,
    video_tag_cnt: usize,
    audio_tag_cnt: usize,
    meta_tag_cnt: usize,
    first_video_dts_nano: u64,
    last_video_dts_nano: u64,
}

impl Gop {
    pub fn new() -> Self {
        Self {
            media_frames: VecDeque::new(),
            video_tag_cnt: 0,
            audio_tag_cnt: 0,
            meta_tag_cnt: 0,
            first_video_dts_nano: 0,
            last_video_dts_nano: 0,
        }
    }

    #[inline]
    pub fn pop_front(&mut self) -> Option<MediaFrame> {
        let dropped = self.media_frames.pop_front();
        if let Some(frame) = dropped.as_ref() {
            if frame.is_audio() {
                self.audio_tag_cnt -= 1;
            } else if frame.is_video() {
                self.video_tag_cnt -= 1;
            }
        }
        self.first_video_dts_nano = self
            .media_frames
            .front()
            .map(|v| v.get_decode_timestamp_ns())
            .unwrap_or(0);
        self.last_video_dts_nano = self
            .media_frames
            .back()
            .map(|v| v.get_decode_timestamp_ns())
            .unwrap_or(0);

        dropped
    }

    #[inline]
    pub fn get_video_frame_cnt(&self) -> usize {
        self.video_tag_cnt
    }

    #[inline]
    pub fn get_audio_frame_cnt(&self) -> usize {
        self.audio_tag_cnt
    }

    #[inline]
    pub fn get_meta_frame_cnt(&self) -> usize {
        self.meta_tag_cnt
    }

    #[inline]
    pub fn get_first_video_dts_nano(&self) -> u64 {
        self.first_video_dts_nano
    }

    #[inline]
    pub fn get_last_video_dts_nano(&self) -> u64 {
        self.last_video_dts_nano
    }

    #[inline]
    pub fn get_last_video_frame_mut(&mut self) -> Option<&mut MediaFrame> {
        self.media_frames
            .iter_mut()
            .rev()
            .find(|frame| frame.is_video())
    }

    pub fn append_media_frame(&mut self, frame: MediaFrame) {
        match &frame {
            MediaFrame::VideoConfig { .. } => {
                self.video_tag_cnt += 1;
            }
            MediaFrame::Video { .. } => {
                self.video_tag_cnt += 1;
                if self.media_frames.is_empty() {
                    self.first_video_dts_nano = frame.get_decode_timestamp_ns();
                }
                self.last_video_dts_nano = frame.get_decode_timestamp_ns();
            }
            MediaFrame::Audio { .. } => self.audio_tag_cnt += 1,
            MediaFrame::AudioConfig { .. } => {
                self.audio_tag_cnt += 1;
            }
            MediaFrame::Script { .. } => self.meta_tag_cnt += 1,
        }

        self.media_frames.push_back(frame);
    }
}

impl Default for Gop {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct GopQueue {
    pub video_config: Option<VideoConfig>, // video config
    pub audio_config: Option<(AudioConfig, SoundInfoCommon)>, // audio config, sound info
    pub script_frame: Option<MediaFrame>,
    pub gops: VecDeque<Gop>,
    total_frame_cnt: u64,
    max_duration_ms: u64,
    max_frame_cnt: u64,
    dropped_gops_cnt: u64,
    dropped_video_cnt: u64,
    dropped_audio_cnt: u64,
}

impl GopQueue {
    pub fn new(max_duration_ms: u64, max_frame_cnt: u64) -> Self {
        Self {
            video_config: None,
            audio_config: None,
            script_frame: None,
            gops: VecDeque::new(),
            max_duration_ms,
            max_frame_cnt,
            total_frame_cnt: 0,
            dropped_gops_cnt: 0,
            dropped_video_cnt: 0,
            dropped_audio_cnt: 0,
        }
    }

    #[inline]
    pub fn get_gops_cnt(&self) -> usize {
        self.gops.len()
    }

    #[inline]
    pub fn get_dropped_gop_cnt(&self) -> u64 {
        self.dropped_gops_cnt
    }

    #[inline]
    pub fn get_dropped_video_cnt(&self) -> u64 {
        self.dropped_video_cnt
    }

    #[inline]
    pub fn get_dropped_audio_cnt(&self) -> u64 {
        self.dropped_audio_cnt
    }

    #[inline]
    fn accumulate_gops<'a, F>(&'a self, f: F) -> usize
    where
        F: Fn(&'a Gop) -> usize,
    {
        let mut result = 0;
        for gop in &self.gops {
            result += f(gop);
        }
        result
    }

    #[inline]
    pub fn get_video_frame_cnt(&self) -> usize {
        self.accumulate_gops(|gop| gop.get_video_frame_cnt())
    }

    #[inline]
    pub fn get_audio_frame_cut(&self) -> usize {
        self.accumulate_gops(|gop| gop.get_audio_frame_cnt())
    }

    #[inline]
    pub fn get_meta_frame_cnt(&self) -> usize {
        self.accumulate_gops(|gop| gop.get_meta_frame_cnt())
    }

    pub fn append_frame(&mut self, mut frame: MediaFrame) -> StreamCenterResult<()> {
        let span = tracing::trace_span!("gop cache append frame");
        let _enter = span.enter();

        if frame.is_video()
            && !frame.is_sequence_header()
            && !frame.is_video_key_frame()
            && self.get_video_frame_cnt() == 0
            && self.max_duration_ms != 0
            && self.max_frame_cnt != 0
        {
            tracing::warn!(
                "first video frame not key frame, dropping. frame codec id: {:?}, timestamp: {}",
                frame.video_codec_id(),
                frame.get_decode_timestamp_ns()
            );
            return Ok(());
        }
        let first_dts = self
            .gops
            .front()
            .map_or(0, |v| v.get_first_video_dts_nano());
        let last_dts = self.gops.back().map_or(0, |v| v.get_last_video_dts_nano());

        if (last_dts > first_dts
            && (last_dts - first_dts) >= self.max_duration_ms.checked_mul(1_000_000).unwrap())
            || self.total_frame_cnt >= self.max_frame_cnt
        {
            let span = tracing::trace_span!(
                "dopping gop",
                last_dts,
                first_dts,
                self.max_duration_ms,
                gops_cnt = self.gops.len(),
                self.total_frame_cnt,
                self.max_frame_cnt
            );
            let _enter = span.enter();
            let dropped = self.gops.pop_front();
            tracing::debug!(
                "dopping a whole gop, frame_cnt={}",
                dropped.as_ref().map_or(0, |v| v.get_meta_frame_cnt())
            );
            if let Some(gop) = dropped {
                self.dropped_gops_cnt += 1;
                self.dropped_video_cnt += gop.get_video_frame_cnt().to_u64().unwrap();
                self.dropped_audio_cnt += gop.get_audio_frame_cnt().to_u64().unwrap();
                self.total_frame_cnt -= gop.media_frames.len().to_u64().unwrap();
            }
        }

        let mut is_sequence_header = false;
        let mut is_video = false;
        match &mut frame {
            MediaFrame::Audio {
                frame_info: _,
                payload: _,
            } => {}
            MediaFrame::VideoConfig {
                timestamp_nano: _,
                config,
            } => {
                self.video_config = Some(*config.clone());
                is_sequence_header = true;
                tracing::info!("got video sh");
            }
            MediaFrame::AudioConfig {
                timestamp_nano: _,
                sound_info,
                config,
            } => {
                self.audio_config = Some((*config.clone(), *sound_info));
                is_sequence_header = true;
            }
            MediaFrame::Video {
                frame_info,
                payload: _,
            } => {
                is_video = true;
                if frame_info.frame_type == FrameType::KeyFrame {
                    self.gops.push_back(Gop::new());
                }
            }
            MediaFrame::Script {
                timestamp_nano: pts,
                on_meta_data,
                payload,
            } => {
                self.script_frame = Some(MediaFrame::Script {
                    timestamp_nano: *pts,
                    on_meta_data: on_meta_data.clone(),
                    payload: payload.clone(),
                });
                is_sequence_header = true;
                tracing::info!("meta, pts: {}, data: {:?}", pts, on_meta_data);
            }
        }

        if is_sequence_header {
            tracing::trace!("skip sequence header");
            return Ok(());
        }

        if self.gops.is_empty() && is_video {
            self.dropped_video_cnt += 1;
            return Ok(());
        }

        if self.gops.is_empty() {
            self.gops.push_back(Gop::new());
        }

        self.gops
            .back_mut()
            .expect("this cannot be empty")
            .append_media_frame(frame);
        self.total_frame_cnt += 1;

        Ok(())
    }
}
