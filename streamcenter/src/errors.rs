use std::{backtrace::Backtrace, io};

use thiserror::Error;

use crate::stream_source::StreamIdentifier;
#[derive(Debug, Error)]
pub enum StreamCenterError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("stream is already publishing {0:?}")]
    DuplicateStream(StreamIdentifier),
    #[error("stream not found: {0:?}")]
    StreamNotFound(StreamIdentifier),
    #[error("channel send failed, {backtrace}")]
    ChannelSendFailed { backtrace: Backtrace },
    #[error("invalid stream type: {0}")]
    InvalidStreamType(String),
    #[error("parse flv tag failed")]
    ParseFLVTagFailed(#[from] flv::errors::FLVError),
}

pub type StreamCenterResult<T> = Result<T, StreamCenterError>;
