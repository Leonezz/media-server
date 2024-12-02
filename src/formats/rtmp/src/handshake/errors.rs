use std::{io, time};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DigestError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("wrong digest length: {length}")]
    WrongLength { length: usize },
    #[error("generate failed")]
    GenerateFailed,
    #[error("unknown schema: {0}")]
    UnknownSchema(u8),
    #[error("digest not found")]
    NotFound,
}

#[derive(Debug, Error)]
pub enum HandshakeError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("system time error: {0}")]
    SystemTimeError(#[from] time::SystemTimeError),
    #[error("digest error: {0}")]
    DigestError(#[from] DigestError),
    #[error("s0 version not match")]
    S0VersionNotMatch,
    #[error("bad version: {0}")]
    BadVersion(u8),
}

pub type HandshakeResult<T> = Result<T, HandshakeError>;
