use std::fmt;

use tokio_util::bytes::Bytes;

use super::{
    audio_tag_header::AudioTagHeader,
    encryption::{EncryptionTagHeader, FilterParams},
    video_tag_header::VideoTagHeader,
};

pub mod reader;
pub mod writer;

#[derive(Debug)]
pub struct Filter {
    encryption_header: EncryptionTagHeader,
    filter_params: FilterParams,
}

pub enum FLVTagBody {
    Audio {
        header: AudioTagHeader,
        body: Bytes,
    },
    Video {
        header: VideoTagHeader,
        body: Bytes,
    },
    Script {
        value: Vec<amf_formats::amf0::Value>,
    },
}

impl fmt::Debug for FLVTagBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FLVTagBody::Audio { header, body } => f.write_fmt(format_args!(
                "Audio tag body, header: {:?}, payload length: {}",
                header,
                body.len()
            )),
            FLVTagBody::Video { header, body } => f.write_fmt(format_args!(
                "Video tag body, header: {:?}, payload length: {}",
                header,
                body.len()
            )),
            FLVTagBody::Script { value } => {
                f.write_fmt(format_args!("Meta tag body, value: {:?}", value))
            }
        }
    }
}

#[derive(Debug)]
pub struct FLVTagBodyWithFilter {
    pub filter: Option<Filter>,
    pub body: FLVTagBody,
}
