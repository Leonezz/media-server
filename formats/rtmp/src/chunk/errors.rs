use std::{backtrace::Backtrace, io, time::SystemTimeError};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChunkMessageError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("unexpected fmt bits: {0:#b}")]
    UnexpectedFmt(u8),
    #[error("unknown message type: {type_id}, backtrace: {backtrace}")]
    UnknownMessageType { type_id: u8, backtrace: Backtrace },
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
    #[error("unexpected amf type: {amf_type}, backtrace: {backtrace}")]
    UnexpectedAmfType {
        amf_type: String,
        backtrace: Backtrace,
    },
    #[error("unexpected command name: {0}")]
    UnexpectedCommandName(String),
    #[error("unknown amf version: {0}")]
    UnknownAmfVersion(u8),
    #[error("error while read or write meta data message: {0}")]
    MetaDataError(#[from] amf::errors::AmfError),
    #[error("get system time failed: {0}, this is wired")]
    SystemTimeError(#[from] SystemTimeError),
    #[error("not error, just not a full chunk message")]
    IncompleteChunk,
}

pub type ChunkMessageResult<T> = Result<T, ChunkMessageError>;
