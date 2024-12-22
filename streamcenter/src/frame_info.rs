use std::fmt::Debug;

use tokio_util::bytes::BytesMut;

#[derive(Debug, Clone, Copy, Default)]
pub struct VideoResolution {
    width: usize,
    height: usize,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MediaMessageRuntimeStat {
    pub read_time_ns: u128,
    pub session_process_time_ns: u128,
    pub publish_stream_source_time_ns: u128,
    pub stream_source_received_time_ns: u128,
    pub stream_source_parse_time_ns: u128,
    pub play_time_ns: u128,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct VideoMeta {
    pub pts: u64,
    // NOTE - this tag_header is also included in the frame payload
    pub tag_header: flv::tag::video_tag_header::VideoTagHeader,
    pub resolution: VideoResolution,

    pub runtime_stat: MediaMessageRuntimeStat,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MetaMeta {
    pub pts: u64,

    pub runtime_stat: MediaMessageRuntimeStat,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AudioMeta {
    pub pts: u64,
    // NOTE - this tag_header is also included in the frame payload
    pub tag_header: flv::tag::audio_tag_header::AudioTagHeader,

    pub runtime_stat: MediaMessageRuntimeStat,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AggregateMeta {
    pub pts: u64,
    pub read_time_ns: u128,
    pub session_process_time_ns: u128,
    pub publish_stream_source_time_ns: u128,
}

#[derive(Debug, Clone)]
pub enum FrameData {
    Video { meta: VideoMeta, payload: BytesMut },
    Audio { meta: AudioMeta, payload: BytesMut },
    Aggregate { meta: AggregateMeta, data: BytesMut },
    Meta { meta: MetaMeta, payload: BytesMut },
}

impl FrameData {
    pub fn log_runtime_stat(&self) {
        match self {
            FrameData::Video { meta, payload: _ } => {
                tracing::info!("video message stat: {:?}", meta.runtime_stat);
            }
            FrameData::Audio { meta, payload: _ } => {
                tracing::info!("audio message stat: {:?}", meta.runtime_stat);
            }
            FrameData::Meta { meta, payload: _ } => {
                tracing::info!("meta message stat: {:?}", meta.runtime_stat);
            }
            _ => {}
        }
    }
}

impl FrameData {
    #[inline]
    pub fn is_video(&self) -> bool {
        match self {
            FrameData::Video {
                meta: _,
                payload: _,
            } => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_audio(&self) -> bool {
        match self {
            FrameData::Audio {
                meta: _,
                payload: _,
            } => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_meta(&self) -> bool {
        match self {
            FrameData::Meta {
                meta: _,
                payload: _,
            } => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_video_key_frame(&self) -> bool {
        match self {
            FrameData::Video { meta, payload: _ } => meta.tag_header.is_key_frame(),
            _ => false,
        }
    }

    #[inline]
    pub fn is_sequence_header(&self) -> bool {
        match self {
            FrameData::Audio { meta, payload: _ } => meta.tag_header.is_sequence_header(),
            FrameData::Video { meta, payload: _ } => meta.tag_header.is_sequence_header(),
            _ => false,
        }
    }
}
