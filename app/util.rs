use config::ConfigError;

use crate::errors::{AppError, AppResult};

pub(crate) fn parse_log_level(level: &str) -> AppResult<tracing::Level> {
    match level.to_lowercase().as_str() {
        "trace" => Ok(tracing::Level::TRACE),
        "debug" => Ok(tracing::Level::DEBUG),
        "info" => Ok(tracing::Level::INFO),
        "warn" => Ok(tracing::Level::WARN),
        "error" => Ok(tracing::Level::ERROR),
        other => Err(AppError::ConfigError(ConfigError::Message(format!(
            "got unexpected log level: {}",
            other
        )))),
    }
}
