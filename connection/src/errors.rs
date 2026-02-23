use thiserror::Error;
#[derive(Debug, Error)]
pub enum ConnError {
    #[error("io error")]
    Io(#[from] std::io::Error),
}

pub type ConnResult<T> = Result<T, ConnError>;