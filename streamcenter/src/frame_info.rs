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
    pts: u64,
    dts: u64,
    codec: VideoCodec,
    resolution: VideoResolution,
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
    Meta { timestamp: u32, data: BytesMut },
}
