use std::{env, net::IpAddr, path::PathBuf};

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

use crate::{
    AppCli,
    errors::{AppError, AppResult},
    util::parse_log_level,
};

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub(crate) struct Logger {
    pub(crate) level: String,
    pub(crate) dir: PathBuf,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub(crate) struct RtmpServer {
    pub(crate) enable: bool,
    pub(crate) address: IpAddr,
    pub(crate) port: u16,
    pub(crate) chunk_size: u32,
    pub(crate) write_timeout_ms: u64,
    pub(crate) read_timeout_ms: u64,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub(crate) struct HttpServer {
    pub(crate) enable: bool,
    pub(crate) address: IpAddr,
    pub(crate) port: u16,
    pub(crate) workers: u64,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub(crate) struct AppConfig {
    pub(crate) logger: Logger,
    pub(crate) rtmp_server: RtmpServer,
    pub(crate) http_server: HttpServer,
}

impl AppConfig {
    pub(crate) fn new(config_path: Option<String>) -> AppResult<Self> {
        let config_path_composed = config_path
            .map(|v| v.to_owned())
            .or_else(|| env::var("YAM_CONFIG").ok());
        if config_path_composed.is_none() {
            return Err(AppError::ConfigError(ConfigError::NotFound(
                "no config file is provided".to_owned(),
            )));
        }
        let result = Config::builder()
            .add_source(File::with_name(config_path_composed.unwrap().as_str()))
            .add_source(Environment::with_prefix("yam"))
            .build()?;
        let config = result.try_deserialize()?;
        Ok(config)
    }

    pub(crate) fn apply(&mut self, cli_args: AppCli) -> AppResult<()> {
        if cli_args.log_level.is_some() {
            self.logger.level = cli_args.log_level.unwrap();
        }

        if cli_args.rtmp_port.is_some() {
            self.rtmp_server.port = cli_args.rtmp_port.unwrap();
        }

        if cli_args.http_port.is_some() {
            self.http_server.port = cli_args.http_port.unwrap();
        }

        Ok(())
    }

    pub(crate) fn validate(&self) -> AppResult<()> {
        let _ = parse_log_level(&self.logger.level)?;

        if self.logger.dir.clone().into_os_string().is_empty() {
            return Err(AppError::ConfigError(ConfigError::Message(format!(
                "the log dir config is empty: {:?}",
                self.logger.dir.to_str()
            ))));
        }

        Ok(())
    }
}
