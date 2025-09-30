use std::process;

use clap::Parser;

use crate::logger::{read_logger_config, setup_logger};
mod errors;
mod logger;
#[tokio::main]
async fn main() {
    let cli = yam_server::cli::AppCli::parse();
    let config_path = cli.config.clone().map(|v| v.to_string_lossy().to_string());
    let config = yam_server::config::AppConfig::new(config_path.clone());

    if config.is_err() {
        eprintln!("app config is not valid: {}", config.unwrap_err());
        process::exit(1);
    }

    let mut config = config.unwrap();
    let logger_config = read_logger_config(config_path);
    if let Err(err) = logger_config.as_ref() {
        eprintln!("read logger config failed: {}", err);
    }
    let logger_config = logger_config.unwrap_or_default();
    setup_logger(format!("{},hyper=off", logger_config.level), logger_config);
    config.apply(cli).unwrap();
    let validate_res = config.validate();
    if validate_res.is_err() {
        panic!(
            "config is not valid: {}.\nconfig is: {:?}",
            validate_res.unwrap_err(),
            config
        );
    }

    yam_server::app_run(config, signal::stop()).await;
}
