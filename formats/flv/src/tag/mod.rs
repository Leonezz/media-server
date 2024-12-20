use std::fmt::Debug;

use audio_tag_header::AudioTagHeader;
use encryption::{EncryptionTagHeader, FilterParams};
use tokio_util::bytes::BytesMut;
use video_tag_header::VideoTagHeader;

use crate::errors::FLVError;

pub mod audio_tag_header;
pub mod encryption;
pub mod reader;
pub mod video_tag_header;
pub mod writer;
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum FLVTagType {
    Audio = 8,
    Video = 9,
    Meta = 18,
}

impl Into<u8> for FLVTagType {
    fn into(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for FLVTagType {
    type Error = FLVError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            8 => Ok(FLVTagType::Audio),
            9 => Ok(FLVTagType::Video),
            18 => Ok(FLVTagType::Meta),
            _ => Err(FLVError::UnknownFLVTagType(value)),
        }
    }
}

#[derive(Debug)]
pub struct FLVTagBodyWithFilter {
    filter: Option<Filter>,
    body: FLVTagBody,
}

#[derive(Debug)]
pub struct FLVTag {
    tag_type: FLVTagType,
    data_size: u32,
    timestamp: u32,
    // stream_id: u32, // always 0
    body_with_filter: FLVTagBodyWithFilter,
}

#[derive(Debug)]
pub struct Filter {
    encryption_header: EncryptionTagHeader,
    filter_params: FilterParams,
}

pub enum FLVTagBody {
    Audio {
        header: AudioTagHeader,
        body: BytesMut,
    },
    Video {
        header: VideoTagHeader,
        body: BytesMut,
    },
    Meta {
        /// Method or object name.
        /// SCRIPTDATAVALUE.Type = 2 (String)
        name: String,
        /// AMF arguments or object properties.
        /// SCRIPTDATAVALUE.Type = 8 (ECMA array)
        value: Vec<(String, amf::amf0::Value)>,
    },
}

impl Debug for FLVTagBody {
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
            FLVTagBody::Meta { name, value } => f.write_fmt(format_args!(
                "Meta tag body, name: {}, value: {:?}",
                name, value
            )),
        }
    }
}
