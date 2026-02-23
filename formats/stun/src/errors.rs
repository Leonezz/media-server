use thiserror::Error;
#[derive(Debug, Error)]
pub enum STUNMessageError {
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Syntax error: {0}")]
    SyntaxError(String),
}
