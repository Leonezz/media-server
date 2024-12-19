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
        stream_center_event_sender: mpsc::UnboundedSender<StreamCenterEvent>,
        config: &RtmpServerConfig,
    ) -> Self {
        Self {
            config: config.clone(),
            stream_center_event_sender,
        }
    }

    #[instrument]
    pub async fn run(&mut self) -> RtmpServerResult<()> {
        let listener = tokio::net::TcpListener::bind(("0.0.0.0", self.config.port)).await?;
        loop {
            let (tcp_stream, addr) = listener.accept().await?;
            tracing::info!("{}", addr);
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
                        tracing::info!("session successfully closed");
                    }
                    Err(err) => {
                        tracing::error!("{:?}", err);
                    }
                };
            });
        }
    }
}
