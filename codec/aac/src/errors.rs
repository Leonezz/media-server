use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AACCodecError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("unknown aac object type id: {0}")]
    UnknownAACObjectTypeId(u8),
    #[error("unknown aac sampling frequency index: {0}")]
    UnknownAACSamplingFrequencyIndex(u8),
    #[error("unknown HVXCrateMode: {0}")]
    UnknownHVXCrateMode(u8),
    #[error("unknown 2bits ObjectType: {0}")]
    UnknownObjectType(u8),
    #[error("unknwn token for orch_token: {0}")]
    UnknownOrchToken(u8),
    #[error("unknwn event type for score_line: {0}")]
    UnknownScoreLineType(u8),
}

pub type AACCodecResult<T> = Result<T, AACCodecError>;
