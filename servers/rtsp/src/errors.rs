use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RTSPMessageError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("Unknown method: {0}")]
    UnknownMethod(String),
    #[error("Unknown header: {0}")]
    UnknownHeader(String),
    #[error("Unknown status code: {0}")]
    UnknownStatusCode(u16),
}
