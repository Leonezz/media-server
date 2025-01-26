use crate::errors::FLVError;

pub mod ex_audio;
pub mod ex_video;

pub const fn make_four_cc(cc: &str) -> u32 {
    assert!(cc.len() == 4);
    let bytes = cc.as_bytes();

    (bytes[0] as u32) << 24 | (bytes[1] as u32) << 16 | (bytes[2] as u32) << 8 | (bytes[3] as u32)
}

/// Used by audio and video pipeline
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvMultiTrackType {
    OneTrack = 0,
    ManyTracks = 1,
    ManyTracksManyCodecs = 2,
}

impl From<AvMultiTrackType> for u8 {
    fn from(value: AvMultiTrackType) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for AvMultiTrackType {
    type Error = FLVError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::OneTrack),
            1 => Ok(Self::ManyTracks),
            2 => Ok(Self::ManyTracksManyCodecs),
            _ => Err(FLVError::UnknownMultiTrackType(value)),
        }
    }
}
