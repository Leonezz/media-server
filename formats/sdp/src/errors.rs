use std::{fmt, io, num::ParseIntError};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SDPError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("fmt error: {0}")]
    FMTError(#[from] fmt::Error),
    #[error("invalid payload")]
    InvalidPayload(String),
    #[error("parse string to integer failed: {0}")]
    ParseToIntegerFailed(#[from] ParseIntError),
    #[error("integer overflow: {0}")]
    IntegerOverflow(String),
    #[error("parse url failed: {0}")]
    ParseUrlFailed(#[from] url::ParseError),
    #[error("syntax error: {0}")]
    SyntaxError(String),
}

pub type SDPResult<T> = Result<T, SDPError>;
