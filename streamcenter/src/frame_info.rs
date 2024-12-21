use std::fmt::Debug;

use tokio_util::bytes::BytesMut;

#[derive(Debug, Clone, Copy, Default)]
pub struct VideoResolution {
    width: usize,
    height: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct VideoMeta {
    pub pts: u64,
    pub dts: u64,
    // NOTE - this tag_header is also included in the frame payload
    pub tag_header: flv::tag::video_tag_header::VideoTagHeader,
    pub resolution: VideoResolution,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AudioMeta {
    pub pts: u64,
    pub dts: u64,
    // NOTE - this tag_header is also included in the frame payload
    pub tag_header: flv::tag::audio_tag_header::AudioTagHeader,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AggregateMeta {
    pub pts: u64,
}

#[derive(Debug, Clone)]
pub enum FrameData {
    Video { meta: VideoMeta, data: BytesMut },
    Audio { meta: AudioMeta, data: BytesMut },
    Aggregate { meta: AggregateMeta, data: BytesMut },
    Meta { timestamp: u32, data: amf::Value },
}

impl FrameData {
    #[inline]
    pub fn is_video(&self) -> bool {
        match self {
            FrameData::Video { meta: _, data: _ } => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_audio(&self) -> bool {
        match self {
            FrameData::Audio { meta: _, data: _ } => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_meta(&self) -> bool {
        match self {
            FrameData::Meta {
                timestamp: _,
                data: _,
            } => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_video_key_frame(&self) -> bool {
        match self {
            FrameData::Video { meta, data: _ } => meta.tag_header.is_key_frame(),
            _ => false,
        }
    }

    #[inline]
    pub fn is_sequence_header(&self) -> bool {
        match self {
            FrameData::Audio { meta, data: _ } => meta.tag_header.is_sequence_header(),
            FrameData::Video { meta, data: _ } => meta.tag_header.is_sequence_header(),
            _ => false,
        }
    }
}
