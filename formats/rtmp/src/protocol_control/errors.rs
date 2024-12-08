use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtocolControlMessageRWError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid message: {0}")]
    InvalidMessage(String),
    #[error("unknown message type: {0}")]
    UnknownMessageType(u8),
}

pub type ProtocolControlMessageRWResult<T> = Result<T, ProtocolControlMessageRWError>;
