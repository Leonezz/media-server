use thiserror::Error;

#[derive(Debug, Error)]
pub enum UnifiedIOError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type UnifiedIOResult<T> = Result<T, UnifiedIOError>;
