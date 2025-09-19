use config::ConfigError;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum AppError {
    #[error("config error: {0}")]
    ConfigError(#[from] ConfigError),
}

pub(crate) type AppResult<T> = Result<T, AppError>;
