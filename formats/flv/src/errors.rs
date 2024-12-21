use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FLVError {
    #[error("Io error: {0}")]
    Io(#[from] io::Error),
    #[error("unknown signature: {0:?}")]
    UnknownSignature([u8; 3]),
    #[error("unknown flv tag type: {0}")]
    UnknownFLVTagType(u8),
    #[error("unknown audio sound format: {0}")]
    UnknownAudioSoundFormat(u8),
    #[error("unknown audio sound rate: {0}")]
    UnknownAudioSoundRate(u8),
    #[error("inconsistent header: {0}")]
    InconsistentHeader(String),
    #[error("unknown video frame type: {0}")]
    UnknownVideoFrameType(u8),
    #[error("unknown video codec id: {0}")]
    UnknownCodecID(u8),
    #[error("unknown avc packet type: {0}")]
    UnknownAVCPacketType(u8),
    #[error("amf meta error: {0:?}")]
    AMFError(#[from] amf::errors::AmfError),
    #[error("unexpected value: {0}")]
    UnexpectedValue(String),
}

pub type FLVResult<T> = Result<T, FLVError>;
