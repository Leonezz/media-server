use std::net::{IpAddr, Ipv4Addr};

use clap::Parser;
use iana_formats::service_port::ServicePortStatic;
use serde::{Deserialize, Serialize};

const DEFAULT_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
const DEFAULT_RTMP_PORT: u16 = iana_formats::service_port::RTMP_PORT_TCP::PORT.min_port();
const DEFAULT_HTTP_PORT: u16 = iana_formats::service_port::HTTP_TCP::PORT.min_port();
const DEFAULT_RTSP_PORT: u16 = iana_formats::service_port::RTSP_TCP::PORT.min_port();

#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub(crate) struct RtmpServer {
    #[arg(
        long = "enable-rtmp",
        name = "enable-rtmp",
        env = "ENABLE_RTMP",
        value_name = "ENABLE",
        help = "enable rtmp server or not",
        default_value = "true"
    )]
    pub(crate) enable: bool,
    #[arg(
        long = "rtmp-address",
        name = "rtmp-address",
        env = "RTMP_ADDRESS",
        value_name = "ADDRESS",
        help = "local address to use for rtmp listening",
        ignore_case = true,
        default_value = DEFAULT_ADDRESS.to_string()
    )]
    pub(crate) address: IpAddr,
    #[arg(
        long = "rtmp-port",
        name = "rtmp-port",
        env = "RTMP_PORT",
        value_name = "PORT",
        help = "local port to use for rtmp listening",
        value_parser = clap::value_parser!(u16).range(u16::MIN as i64..u16::MAX as i64),
        ignore_case = true,
        default_value = DEFAULT_RTMP_PORT.to_string()
    )]
    pub(crate) port: u16,
    #[arg(
        long = "rtmp-chunk-size",
        name = "rtmp-chunk-size",
        env = "RTMP_CHUNK_SIZE",
        value_name = "CHUNK_SIZE",
        help = "rtmp chunk size",
        value_parser = clap::value_parser!(u32).range(1000..u32::MAX as i64),
        ignore_case = true,
        default_value = "6000"
    )]
    pub(crate) chunk_size: u32,
    #[arg(
        long = "rtmp-write-timeout-ms",
        name = "rtmp-write-timeout-ms",
        env = "RTMP_WRITE_TIMEOUT_MS",
        value_name = "WRITE_TIMEOUT_MS",
        help = "rtmp write timeout (ms)",
        value_parser = clap::value_parser!(u64),
        ignore_case = true,
        default_value = "6000"
    )]
    pub(crate) write_timeout_ms: u64,
    #[arg(
        long = "rtmp-read-timeout-ms",
        name = "rtmp-read-timeout-ms",
        env = "RTMP_READ_TIMEOUT_MS",
        value_name = "READ_TIMEOUT_MS",
        help = "rtmp read timeout (ms)",
        value_parser = clap::value_parser!(u64),
        ignore_case = true,
        default_value = "6000"
    )]
    pub(crate) read_timeout_ms: u64,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub(crate) struct HttpServer {
    #[arg(
        long = "enable-http",
        name = "enable-http",
        env = "ENABLE_HTTP",
        value_name = "ENABLE",
        help = "enable http server or not",
        default_value = "true"
    )]
    pub(crate) enable: bool,
    #[arg(
        long = "http-address",
        name = "http-address",
        env = "HTTP_ADDRESS",
        value_name = "ADDRESS",
        help = "local address to use for http listening",
        ignore_case = true,
        default_value = DEFAULT_ADDRESS.to_string()
    )]
    pub(crate) address: IpAddr,
    #[arg(
        long = "http-port",
        name = "http-port",
        env = "HTTP_PORT",
        value_name = "PORT",
        help = "local port to use for http listening",
        value_parser = clap::value_parser!(u16).range(u16::MIN as i64..u16::MAX as i64),
        ignore_case = true,
        default_value = DEFAULT_HTTP_PORT.to_string()
    )]
    pub(crate) port: u16,
    #[arg(
        long = "http-workers",
        name = "http-workers",
        env = "HTTP_WORKERS",
        value_name = "WORKERS",
        help = "number of work threads to use for http server",
        value_parser = clap::value_parser!(u64),
        ignore_case = true,
        default_value = "10"
    )]
    pub(crate) workers: u64,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
pub(crate) struct RtspServer {
    #[arg(
        long = "enable-rtsp",
        name = "enable-rtsp",
        env = "ENABLE_RTSP",
        value_name = "ENABLE",
        help = "enable rtsp server or not",
        default_value = "true"
    )]
    pub(crate) enable: bool,
    #[arg(
        long = "rtsp-address",
        name = "rtsp-address",
        env = "RTSP_ADDRESS",
        value_name = "ADDRESS",
        ignore_case = true,
        help = "local address to use for rtsp listening",
        value_parser = clap::value_parser!(IpAddr),
        default_value = DEFAULT_ADDRESS.to_string()
    )]
    pub(crate) address: IpAddr,
    #[arg(
        long = "rtsp-port",
        name = "rtsp-port",
        env = "RTSP_PORT",
        value_name = "PORT",
        help = "local port to use for rtsp listening",
        value_parser = clap::value_parser!(u16).range(u16::MIN as i64..u16::MAX as i64),
        ignore_case = true,
        default_value = DEFAULT_RTSP_PORT.to_string()
    )]
    pub(crate) port: u16,
}

#[derive(Debug, Parser, Serialize, Deserialize, Default)]
pub struct YamServerCli {
    #[command(flatten)]
    pub(crate) rtmp_server: Option<RtmpServer>,
    #[command(flatten)]
    pub(crate) http_server: Option<HttpServer>,
    #[command(flatten)]
    pub(crate) rtsp_server: Option<RtspServer>,
}

impl std::fmt::Display for YamServerCli {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub type AppCli = utils::config::AppCli<utils::config::AppCliWithLogger<YamServerCli>>;
