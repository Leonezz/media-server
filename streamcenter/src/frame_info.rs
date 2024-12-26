use std::fmt::Debug;

use flv::tag::{
    audio_tag_header,
    enhanced::{
        ex_audio::ex_audio_header::ExAudioTagHeader, ex_video::ex_video_header::ExVideoTagHeader,
    },
    video_tag_header,
};
use tokio_util::{bytes::BytesMut, either::Either};

#[derive(Debug, Default, Clone, Copy)]
pub struct MediaMessageRuntimeStat {
    pub read_time_ns: u128,
    pub session_process_time_ns: u128,
    pub publish_stream_source_time_ns: u128,
    pub stream_source_received_time_ns: u128,
    pub stream_source_parse_time_ns: u128,
    pub play_time_ns: u128,
}

#[derive(Debug, Clone)]
pub struct VideoMeta {
    pub pts: u64,
    // NOTE - this tag_header is also included in the frame payload
    pub tag_header: Either<flv::tag::video_tag_header::VideoTagHeader, ExVideoTagHeader>,

    pub runtime_stat: MediaMessageRuntimeStat,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MetaMeta {
    pub pts: u64,

    pub runtime_stat: MediaMessageRuntimeStat,
}

#[derive(Debug, Clone)]
pub struct AudioMeta {
    pub pts: u64,
    // NOTE - this tag_header is also included in the frame payload
    pub tag_header: Either<flv::tag::audio_tag_header::AudioTagHeader, ExAudioTagHeader>,

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
            FrameData::Video { meta, payload: _ } => match &meta.tag_header {
                Either::Left(header) => header.is_key_frame(),
                Either::Right(ex_header) => ex_header.is_key_frame(),
            },
            _ => false,
        }
    }

    #[inline]
    pub fn is_sequence_header(&self) -> bool {
        match self {
            FrameData::Audio { meta, payload: _ } => {
                audio_tag_header::is_sequence_header(&meta.tag_header)
            }
            FrameData::Video { meta, payload: _ } => {
                video_tag_header::is_sequence_header(&meta.tag_header)
            }
            _ => false,
        }
    }
}
