use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ChunkMessageError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("unexpected fmt bits: {0:#b}")]
    UnexpectedFmt(u8),
    #[error("unknown message type: {0}")]
    UnknownMessageType(u8),
    #[error("invalid csid: {0}")]
    InvalidBasicHeader(String),
    #[error("invalid message header: {0}")]
    InvalidMessageHead(String),
    #[error("message length not match")]
    MessageLengthNotMatch,
    #[error("need context for chunk message")]
    NeedContext,
    #[error("failed to insert context")]
    AddContextFailed,
}

pub type ChunkMessageResult<T> = Result<T, ChunkMessageError>;
