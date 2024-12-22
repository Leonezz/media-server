use stream_center::events::StreamCenterEvent;
use tokio::sync::mpsc;
use tracing::instrument;

use crate::config::RtmpSessionConfig;

use super::{config::RtmpServerConfig, errors::RtmpServerResult, session::RtmpSession};

#[derive(Debug)]
pub struct RtmpServer {
    config: RtmpServerConfig,
    stream_center_event_sender: mpsc::UnboundedSender<StreamCenterEvent>,
}

impl RtmpServer {
    pub fn new(
        config: &RtmpServerConfig,
        stream_center_event_sender: mpsc::UnboundedSender<StreamCenterEvent>,
    ) -> Self {
        Self {
            config: config.clone(),
            stream_center_event_sender,
        }
    }

    pub async fn run(&mut self) -> RtmpServerResult<()> {
        tracing::info!("rtmp server is running: {:?}", self.config);
        let listener =
            tokio::net::TcpListener::bind((self.config.ip.as_str(), self.config.port)).await?;
        loop {
            let (tcp_stream, addr) = listener.accept().await?;
            let peer_addr = tcp_stream.peer_addr();
            tracing::info!(
                "got new rtmp connection, addr: {}, peer addr: {:?}",
                addr,
                peer_addr
            );
            let mut session = RtmpSession::new(
                tcp_stream,
                self.stream_center_event_sender.clone(),
                RtmpSessionConfig {
                    chunk_size: self.config.chunk_size,
                    write_timeout_ms: self.config.write_timeout_ms,
                    read_timeout_ms: self.config.read_timeout_ms,
                },
            );
            tokio::spawn(async move {
                match session.run().await {
                    Ok(()) => {
                        tracing::info!(
                            "rtmp session successfully closed, addr: {}, peer addr: {:?}",
                            addr,
                            peer_addr
                        );
                    }
                    Err(err) => {
                        tracing::error!("{:?}", err);
                    }
                };
                session.log_stats().await;
            });
        }
    }
}
