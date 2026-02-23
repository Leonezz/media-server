use http_server::{config::HttpServerConfig, server::HttpServer};
use rtsp_server::server::RtspServer;
use stream_center::stream_center;
use tracing::{self};
pub mod config;
use config::AppConfig;
pub mod cli;
mod errors;

pub async fn app_run<F>(config: AppConfig, stop: F)
where
    F: Future<Output = ()> + Send + 'static,
{
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
    stop.await;
}
