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
    AMFError(#[from] amf_formats::errors::AmfError),
    #[error("unexpected value: {0}")]
    UnexpectedValue(String),
    #[error("unknown fourcc: {0}")]
    UnknownFourCC(String),
    #[error("unknown audio packet type: {0}")]
    UnknownAudioPacketType(u8),
    #[error("unknown multi track type: {0}")]
    UnknownMultiTrackType(u8),
    #[error("unknown audio channel order: {0}")]
    UnknownChannelOrder(u8),
    #[error("unknown audio channel: {0}")]
    UnknownAudioChannel(u8),
    #[error("unknown audio packet mod ex type: {0}")]
    UnknownAudioPacketModExType(u8),
    #[error("unknown video packet mod ex type: {0}")]
    UnknownVideoPacketModExType(u8),
    #[error("unknown video command type: {0}")]
    UnknownVideoCommandType(u8),
    #[error("unknown video packet type: {0}")]
    UnknownVideoPacketType(u8),
}

pub type FLVResult<T> = Result<T, FLVError>;
