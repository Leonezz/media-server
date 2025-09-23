use std::{env, net::IpAddr};

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

use crate::{
    cli::AppCli,
    errors::{AppError, AppResult},
};

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
pub(crate) struct RtspServer {
    pub(crate) enable: bool,
    pub(crate) address: IpAddr,
    pub(crate) port: u16,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct AppConfig {
    pub(crate) rtmp_server: RtmpServer,
    pub(crate) http_server: HttpServer,
    pub(crate) rtsp_server: RtspServer,
}

impl AppConfig {
    pub fn new(config_path: Option<String>) -> AppResult<Self> {
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

    pub fn apply(&mut self, cli_args: AppCli) -> AppResult<()> {
        if cli_args.rtmp_port.is_some() {
            self.rtmp_server.port = cli_args.rtmp_port.unwrap();
        }

        if cli_args.http_port.is_some() {
            self.http_server.port = cli_args.http_port.unwrap();
        }

        if cli_args.rtsp_port.is_some() {
            self.rtsp_server.port = cli_args.rtsp_port.unwrap();
        }
        Ok(())
    }

    pub fn validate(&self) -> AppResult<()> {
        Ok(())
    }
}
