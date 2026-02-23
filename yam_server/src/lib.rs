use http_server::server::HttpServer;
use rtsp_server::server::RtspServer;
use stream_center::stream_center;
pub mod cli;

pub async fn app_run<F>(stop: F, config: cli::YamServerCli)
where
    F: Future<Output = ()> + Send + 'static,
{
    let mut stream_center = stream_center::StreamCenter::new();
    if let Some(rtmp_server) = config.rtmp_server
        && rtmp_server.enable
    {
        let mut server = rtmp_server::server::RtmpServer::new(
            rtmp_server.address,
            rtmp_server.port,
            rtmp_server.chunk_size,
            rtmp_server.write_timeout_ms,
            rtmp_server.read_timeout_ms,
            stream_center.get_event_sender(),
        );
        tokio::spawn(async move {
            if let Err(err) = server.run().await {
                tracing::error!("rtmp server thread exit with err: {:?}", err);
            }
        });

        {
            let msg = format!("rtmp server is started with config: {:?}", rtmp_server);
            tracing::info!(msg);
            println!("{}", msg);
        }
    }

    if let Some(http_server) = config.http_server
        && http_server.enable
    {
        let mut server = HttpServer::new(
            http_server.address,
            http_server.port,
            http_server.workers,
            stream_center.get_event_sender(),
        );
        tokio::spawn(async move {
            if let Err(err) = server.run().await {
                tracing::error!("http server thread exit with err: {:?}", err);
            }
        });

        {
            let msg = format!("http server is started with config: {:?}", http_server);
            tracing::info!(msg);
            println!("{}", msg);
        }
    }

    if let Some(rtsp_server) = config.rtsp_server
        && rtsp_server.enable
    {
        let server = RtspServer::new(
            stream_center.get_event_sender(),
            rtsp_server.address,
            rtsp_server.port,
        );
        tokio::spawn(async move {
            if let Err(err) = server.run().await {
                tracing::error!("rtsp server thread exit with err: {:?}", err);
            }
        });

        {
            let msg = format!("rtsp server is started with config: {:?}", rtsp_server);
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
