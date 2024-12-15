use std::backtrace::Backtrace;

use thiserror::Error;
#[derive(Debug, Error)]
pub enum StreamCenterError {
    #[error("stream is already publishing {0}")]
    DuplicateStream(String),
    #[error("stream not found: {0}")]
    StreamNotFound(String),
    #[error("channel send failed, {backtrace}")]
    ChannelSendFailed { backtrace: Backtrace },
}

pub type StreamCenterResult<T> = Result<T, StreamCenterError>;
