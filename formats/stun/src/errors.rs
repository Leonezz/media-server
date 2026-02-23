use std::string::FromUtf8Error;

use thiserror::Error;
#[derive(Debug, Error)]
pub enum StunMessageError {
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Syntax error: {0}")]
    SyntaxError(String),
    #[error("invalid utf8 string")]
    InvalidUtf8String(#[from] FromUtf8Error),
    #[error("unknown error code: {0:?}")]
    UnknownErrorCode(crate::rfc8489::error_code::ErrorCode),
    #[error("invalid message: {0}")]
    InvalidMessage(String),
}

pub type STUNMessageResult<T> = Result<T, StunMessageError>;
