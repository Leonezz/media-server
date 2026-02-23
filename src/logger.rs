use serde::Deserialize;
use std::path::PathBuf;
use time::macros::format_description;
use tracing_appender::rolling::Rotation;
use tracing_subscriber::{
    EnvFilter, fmt::time::LocalTime, layer::SubscriberExt, util::SubscriberInitExt,
};

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

#[allow(unused)]
pub fn setup_logger(
    env_filter: Option<&str>,
    log_file_prefix: Option<&str>,
    config: &utils::config::LoggerCli,
) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    if config.log_level == utils::config::LogLevel::None {
        return None;
    }

    let time_fmt = if config.log_output == utils::config::LogOutput::FILE {
        LocalTime::new(format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second] [unix_timestamp]"
        ))
    } else {
        LocalTime::new(format_description!("[hour]:[minute][second]"))
    };

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_timer(time_fmt)
        .with_file(true)
        .with_line_number(true)
        .compact()
        .with_ansi(config.log_output != utils::config::LogOutput::FILE);
    let (log_writer, guard) = match config.log_output {
        utils::config::LogOutput::FILE => {
            tracing_appender::non_blocking(tracing_appender::rolling::RollingFileAppender::new(
                Rotation::DAILY,
                config.log_dir.as_ref().unwrap_or(&("logs".into())),
                log_file_prefix.unwrap_or(""),
            ))
        }
        utils::config::LogOutput::STDERR => tracing_appender::non_blocking(std::io::stderr()),
        utils::config::LogOutput::STDOUT => tracing_appender::non_blocking(std::io::stdout()),
    };

    let registry = tracing_subscriber::registry()
        .with(fmt_layer.with_writer(log_writer))
        .with(
            EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new(format!(
                "{},{}",
                config.log_level,
                env_filter.unwrap_or("")
            ))),
        );

    #[cfg(feature = "tokio_unstable")]
    let registry = registry.with(
        console_subscriber::ConsoleLayer::builder()
            .retention(std::time::Duration::from_secs(60))
            .server_addr((127.0.0.1), 6669)
            .spawn(),
    );

    registry.init();
    Some(guard)
}
