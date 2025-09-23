use crate::errors::{AppError, AppResult};
use config::{Environment, File};
use serde::Deserialize;
use std::{env, path::PathBuf};
use time::macros::format_description;
use tracing::Dispatch;
use tracing_appender::rolling::Rotation;
use tracing_subscriber::fmt::time::LocalTime;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct LoggerConfig {
    pub level: String,
    pub dir: PathBuf,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            level: "info".to_owned(),
            dir: "./logs/".into(),
        }
    }
}

pub fn read_logger_config(path: Option<String>) -> AppResult<LoggerConfig> {
    let config_path_composed = path
        .map(|v| v.to_owned())
        .or_else(|| env::var("YAM_CONFIG").ok());
    if config_path_composed.is_none() {
        return Err(AppError::ConfigNotFound("no config provided".to_owned()));
    }
    let result = config::Config::builder()
        .add_source(File::with_name(config_path_composed.unwrap().as_str()))
        .add_source(Environment::with_prefix("yam"))
        .build()?;
    let config = result.try_deserialize()?;
    Ok(config)
}

pub fn setup_logger(env_filter: String, config: LoggerConfig) {
    use tracing_subscriber::EnvFilter;
    let log_writer = tracing_appender::rolling::RollingFileAppender::new(
        Rotation::DAILY,
        config.dir.clone(),
        "yam.log",
    );
    let subscriber = tracing_subscriber::fmt()
        .with_timer(LocalTime::new(format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second] [unix_timestamp]"
        )))
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new(env_filter)))
        .compact()
        .with_ansi(true)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(false)
        .with_target(false)
        .with_writer(log_writer)
        .finish();
    tracing::dispatcher::set_global_default(Dispatch::new(subscriber)).unwrap();
}

pub fn parse_log_level(level: &str) -> AppResult<tracing::Level> {
    match level.to_lowercase().as_str() {
        "trace" => Ok(tracing::Level::TRACE),
        "debug" => Ok(tracing::Level::DEBUG),
        "info" => Ok(tracing::Level::INFO),
        "warn" => Ok(tracing::Level::WARN),
        "error" => Ok(tracing::Level::ERROR),
        _ => Err(AppError::LogLevelNotValid(level.to_owned())),
    }
}

pub fn validate_log_path(path: PathBuf) -> AppResult<()> {
    if path.clone().into_os_string().is_empty() {
        return Err(AppError::LogPathNotValid("path is empty".to_owned()));
    }

    Ok(())
}
