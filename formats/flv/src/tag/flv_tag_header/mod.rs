use utils::traits::fixed_packet::FixedPacket;

use crate::errors::FLVError;

pub mod reader;
pub mod writer;

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum FLVTagType {
    Audio = 8,
    Video = 9,
    Script = 18,
}

impl From<FLVTagType> for u8 {
    fn from(value: FLVTagType) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for FLVTagType {
    type Error = FLVError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            8 => Ok(FLVTagType::Audio),
            9 => Ok(FLVTagType::Video),
            18 => Ok(FLVTagType::Script),
            _ => Err(FLVError::UnknownFLVTagType(value)),
        }
    }
}

#[derive(Debug)]
pub struct FLVTagHeader {
    pub tag_type: FLVTagType,
    pub data_size: u32,
    pub timestamp: u32,
    pub filter_enabled: bool,
    // stream_id: u32, // always 0
}

impl FixedPacket for FLVTagHeader {
    fn bytes_count() -> usize {
        11
    }
}
