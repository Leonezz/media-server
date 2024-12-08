use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RtmpMessageError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("unknown message type: {0}")]
    UnknownMessageType(u8),
    #[error("unknown event type: {0}")]
    UnknownEventType(u16),
}

pub type RtmpMessageResult<T> = Result<T, RtmpMessageError>;
