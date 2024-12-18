use std::fmt::Debug;

use tokio_util::bytes::BytesMut;

#[derive(Debug, Clone, Copy, Default)]
pub enum VideoCodec {
    #[default]
    H264,
    H265,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct VideoResolution {
    width: usize,
    height: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct VideoMeta {
    pub pts: u64,
    pub dts: u64,
    pub codec: VideoCodec,
    pub resolution: VideoResolution,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum AudioCodec {
    #[default]
    AAC,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AudioMeta {
    pts: u64,
    dts: u64,
    codec: AudioCodec,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AggregateMeta {}

#[derive(Debug, Clone)]
pub enum FrameData {
    Video { meta: VideoMeta, data: BytesMut },
    Audio { meta: AudioMeta, data: BytesMut },
    Aggregate { meta: AggregateMeta, data: BytesMut },
    Meta { timestamp: u32, data: amf::Value },
}

impl FrameData {
    pub fn is_video_idr(&self) -> bool {
        false
    }
}
