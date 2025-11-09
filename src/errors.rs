use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("the path for log file is not valid: {0}")]
    LogPathNotValid(String),
    #[error("log level not valid: {0}")]
    LogLevelNotValid(String),
    #[error("config not found: {0}")]
    ConfigNotFound(String),
}

pub type AppResult<T> = Result<T, AppError>;
