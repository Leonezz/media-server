use std::backtrace::Backtrace;

use thiserror::Error;

use crate::stream_source::StreamIdentifier;
#[derive(Debug, Error)]
pub enum StreamCenterError {
    #[error("stream is already publishing {0:?}")]
    DuplicateStream(StreamIdentifier),
    #[error("stream not found: {0:?}")]
    StreamNotFound(StreamIdentifier),
    #[error("channel send failed, {backtrace}")]
    ChannelSendFailed { backtrace: Backtrace },
    #[error("invalid stream type: {0}")]
    InvalidStreamType(String),
}

pub type StreamCenterResult<T> = Result<T, StreamCenterError>;
