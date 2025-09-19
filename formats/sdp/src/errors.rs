use std::{fmt, io};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SDPError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("fmt error: {0}")]
    FMTError(#[from] fmt::Error),
    #[error("invalid payload")]
    InvalidPayload(String),

    #[error("integer overflow: {0}")]
    IntegerOverflow(String),
    #[error("parse url failed: {0}")]
    ParseUrlFailed(#[from] url::ParseError),
    #[error("syntax error: {0}")]
    SyntaxError(String),
    #[error("invalid attribute line: {0}")]
    InvalidAttributeLine(String),
}

pub type SDPResult<T> = Result<T, SDPError>;
