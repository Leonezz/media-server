use std::{io, time::SystemTimeError};

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
    #[error("invalid message body: {0}")]
    InvalidMessage(String),
    #[error("need context for chunk message")]
    NeedContext,
    #[error("failed to insert context")]
    AddContextFailed,
    #[error("unknown event type: {0}")]
    UnknownEventType(u16),
    #[error("unexpected amf type: {0}")]
    UnexpectedAmfType(String),
    #[error("unexpected command name: {0}")]
    UnexpectedCommandName(String),
    #[error("unknown amf version: {0}")]
    UnknownAmfVersion(u8),
    #[error("error while read or write meta data message: {0}")]
    MetaDataError(#[from] amf::errors::AmfError),
    #[error("get system time failed: {0}, this is wired")]
    SystemTimeError(#[from] SystemTimeError),
}

pub type ChunkMessageResult<T> = Result<T, ChunkMessageError>;
