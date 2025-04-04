use unified_io::tcp::TcpIO;

use crate::{config::RtspServerConfig, errors::RtspServerResult, middleware, session::RtspSession};

#[derive(Debug)]
pub struct RtspServer {
    config: RtspServerConfig,
}

impl RtspServer {
    pub fn new(config: RtspServerConfig) -> Self {
        Self { config }
    }

    pub async fn run(&self) -> RtspServerResult<()> {
        tracing::info!("rtsp server is starting with config: {:?}", self.config);
        let listener =
            tokio::net::TcpListener::bind((self.config.address, self.config.port)).await?;
        loop {
            let (tcp_stream, addr) = listener.accept().await?;
            tracing::info!("got new rtsp connection, peer addr: {}", addr);

            let mut session = RtspSession::new(Box::pin(TcpIO::new(tcp_stream)), addr.to_owned())
                .with_middleware(Box::new(
                    middleware::response_header_appender::ResponseHeaderAppender {},
                ));
            tokio::task::spawn(async move {
                match session.run().await {
                    Ok(()) => {
                        tracing::info!("rtsp session gracefully closed, peer addr: {}", addr);
                    }
                    Err(err) => {
                        tracing::error!("rtsp session exit with error: {}", err);
                    }
                };
            });
        }
    }
}
