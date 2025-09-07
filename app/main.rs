use std::env;

use clap::Parser;
use http_server::{config::HttpServerConfig, server::HttpServer};
use rtsp_server::server::RtspServer;
use stream_center::stream_center;
use time::macros::format_description;
use tokio::signal;
use tracing::{self, Dispatch};
use tracing_appender::rolling::Rotation;
use tracing_subscriber::{self, EnvFilter, fmt::time::LocalTime};
mod config;
use config::AppConfig;
mod cli;
mod errors;
use cli::AppCli;
mod util;

#[tokio::main]
async fn main() {
    let cli = AppCli::parse();
    let config = AppConfig::new(cli.config.clone().map(|v| v.to_string_lossy().to_string()));
    match config {
        Err(err) => {
            panic!("parsing app config failed: {}", err);
        }
        Ok(mut config) => {
            config.apply(cli).unwrap();

            let validate_res = config.validate();
            if validate_res.is_err() {
                panic!(
                    "config is not valid: {}.\nconfig is: {:?}",
                    validate_res.unwrap_err(),
                    config
                );
            }

            app_run(config).await;
        }
    }
}

async fn app_run(config: AppConfig) {
    unsafe {
        // we set this special env to disable logs from frameworks
        env::set_var(
            "LOG_LEVEL",
            format!("{},rocket=off,hyper=off", config.logger.level),
        );
        let log_level = env::var("LOG_LEVEL").unwrap();
        println!("set env var LOG_LEVEL to {}", log_level);
    }

    let log_writer = tracing_appender::rolling::RollingFileAppender::new(
        Rotation::DAILY,
        config.logger.dir.clone(),
        "yam.log",
    );
    let subscriber = tracing_subscriber::fmt()
        .with_timer(LocalTime::new(format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second] [unix_timestamp precision:nanosecond]"
        )))
        // Use a more compact, abbreviated log format
        .compact()
        .with_ansi(false)
        // Display source code file paths
        .with_file(true)
        // Display source code line numbers
        .with_line_number(true)
        // Display the thread name an event was recorded on
        // .with_thread_names(true)
        // display the event's target (module path)
        .with_target(false)
        .with_env_filter(EnvFilter::from_env("LOG_LEVEL"))
        .with_writer(log_writer)
        // Build the subscriber
        .finish();
    tracing::dispatcher::set_global_default(Dispatch::new(subscriber)).unwrap();

    {
        let msg = format!("yam_server is starting with config: {:?}", config);
        tracing::info!(msg);
        println!("{}", msg);
    }

    let mut stream_center = stream_center::StreamCenter::new();

    if config.rtmp_server.enable {
        let mut rtmp_server = rtmp_server::server::RtmpServer::new(
            rtmp_server::config::RtmpServerConfig {
                address: config.rtmp_server.address,
                port: config.rtmp_server.port,
                chunk_size: config.rtmp_server.chunk_size,
                write_timeout_ms: config.rtmp_server.write_timeout_ms,
                read_timeout_ms: config.rtmp_server.read_timeout_ms,
            },
            stream_center.get_event_sender(),
        );
        tokio::spawn(async move {
            if let Err(err) = rtmp_server.run().await {
                tracing::error!("rtmp server thread exit with err: {:?}", err);
            }
        });

        {
            let msg = format!(
                "rtmp server is started with config: {:?}",
                config.rtmp_server
            );
            tracing::info!(msg);
            println!("{}", msg);
        }
    }

    if config.http_server.enable {
        let mut http_server = HttpServer::new(
            HttpServerConfig {
                address: config.http_server.address,
                port: config.http_server.port,
                workers: config.http_server.workers,
            },
            stream_center.get_event_sender(),
        );
        tokio::spawn(async move {
            if let Err(err) = http_server.run().await {
                tracing::error!("http server thread exit with err: {:?}", err);
            }
        });

        {
            let msg = format!(
                "http server is started with config: {:?}",
                config.http_server
            );
            tracing::info!(msg);
            println!("{}", msg);
        }
    }

    if config.rtsp_server.enable {
        let rtsp_server = RtspServer::new(
            stream_center.get_event_sender(),
            rtsp_server::config::RtspServerConfig {
                address: config.rtsp_server.address,
                port: config.rtsp_server.port,
            },
        );
        tokio::spawn(async move {
            if let Err(err) = rtsp_server.run().await {
                tracing::error!("rtsp server thread exit with err: {:?}", err);
            }
        });

        {
            let msg = format!(
                "rtsp server is started with config: {:?}",
                config.rtsp_server
            );
            tracing::info!(msg);
            println!("{}", msg);
        }
    }

    tokio::spawn(async move {
        if let Err(err) = stream_center.run().await {
            tracing::error!("stream center thread exit with err: {:?}", err);
        }
    });
    {
        let msg = "stream center is started\nall servers are started".to_string();
        tracing::info!(msg);
        println!("{}", msg);
    }
    let _ = signal::ctrl_c().await;
}
