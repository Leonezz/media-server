use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum H264CodecError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("unknown nalu type: {0}")]
    UnknownNaluType(u8),
    #[error("invalid Exp-Golomb code: {0}")]
    InvalidExpGolombCode(String),
    #[error("syntax error: {0}")]
    SyntaxError(String),
    #[error("unknown video format: {0}")]
    UnknownVideoFormat(u8),
}

pub type H264CodecResult<T> = Result<T, H264CodecError>;
