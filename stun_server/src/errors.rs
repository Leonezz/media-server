use thiserror::Error;
#[derive(Debug, Error)]
pub enum AppError {
    #[error("unknown protocol: {0}")]
    UnknownProtocol(String),
    #[error("config error: {0}")]
    ConfigError(String),
}
