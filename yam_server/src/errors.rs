use config::ConfigError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("config error: {0}")]
    ConfigError(#[from] ConfigError),
}

pub type AppResult<T> = Result<T, AppError>;
