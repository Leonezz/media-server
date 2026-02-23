use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about)]
pub struct AppCli {
    #[arg(short, long, value_name = "CONFIG_FILE")]
    pub config: Option<PathBuf>,
    #[arg(long, value_name = "RTMP_PORT")]
    pub(crate) rtmp_port: Option<u16>,
    #[arg(long, value_name = "HTTP_PORT")]
    pub(crate) http_port: Option<u16>,
    #[arg(long, value_name = "RTSP_PORT")]
    pub(crate) rtsp_port: Option<u16>,
}
