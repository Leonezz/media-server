use thiserror::Error;

use crate::audio::AudioConfig;
#[derive(Debug, Error)]
pub enum CodecCommonError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid codec id: {0}")]
    InvalidCodecId(u8),
    #[error("invalid nalu size length minus one: {0}")]
    InvalidNaluSizeLengthMinueOne(u8),
    #[error("parse h264 nalu failed: {0}")]
    ParseH264NaluFailed(#[from] codec_h264::errors::H264CodecError),
    #[error("write audio config failed: {0:?}, {1}")]
    WriteAudioConfigFailed(Box<AudioConfig>, String),
}
pub type CodecCommonResult<T> = Result<T, CodecCommonError>;
