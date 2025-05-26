use thiserror::Error;
#[derive(Debug, Error)]
pub enum StreamPropertiesError {
    #[error("parse from url failed: {0}")]
    ParseFromUrlFailed(String),
}

pub type StreamPropertiesResult<T> = Result<T, StreamPropertiesError>;
