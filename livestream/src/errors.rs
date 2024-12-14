use thiserror::Error;
#[derive(Debug, Error)]
pub enum StreamCenterError {
    #[error("stream is already publishing {0}")]
    DuplicateStream(String),
}

pub type StreamCenterResult<T> = Result<T, StreamCenterError>;
