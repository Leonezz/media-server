use thiserror::Error;
#[derive(Debug, Error)]
pub enum AppError {
    #[error("unknown protocol: {0}")]
    UnknownProtocol(String),
    #[error("invalid server: {0}")]
    InvalidServer(String),
    #[error("config error: {0}")]
    ConfigError(String),
}
