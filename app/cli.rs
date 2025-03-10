use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about)]
pub(crate) struct AppCli {
  #[arg(short, long, value_name = "CONFIG_FILE")]
  pub(crate) config: Option<PathBuf>,
  #[arg(long, value_name = "LOG_LEVEL")]
  pub(crate) log_level: Option<String>,
  #[arg(long, value_name = "RTMP_PORT")]
  pub(crate) rtmp_port: Option<u16>,
  #[arg(long, value_name = "HTTP_PORT")]
  pub(crate) http_port: Option<u16>,
}