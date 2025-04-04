use unified_io::tcp::TcpIO;

use crate::{config::RtspServerConfig, errors::RtspServerResult, session::RtspSession};

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
            let peer_addr = tcp_stream.peer_addr();
            tracing::info!(
                "got new rtsp connection, addr: {}, peer addr: {:?}",
                addr,
                peer_addr
            );

            let mut session = RtspSession::new(Box::pin(TcpIO::new(tcp_stream)));
            let _ = tokio::task::spawn(async move {
                match session.run().await {
                    Ok(()) => {
                        tracing::info!(
                            "rtsp session successfully closed, addr: {}, peer addr: {:?}",
                            addr,
                            peer_addr
                        );
                    }
                    Err(err) => {
                        tracing::error!("{:?}", err);
                    }
                };
            })
            .await;
        }
    }
}
