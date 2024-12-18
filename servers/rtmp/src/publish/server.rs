use stream_center::events::StreamCenterEvent;
use tokio::sync::mpsc;
use tracing::instrument;

use crate::publish::config::RtmpSessionConfig;

use super::{
    config::RtmpPublishServerConfig, errors::RtmpPublishServerResult, session::RtmpPublishSession,
};

#[derive(Debug)]
pub struct RtmpPublishServer {
    config: RtmpPublishServerConfig,
    stream_center_event_sender: mpsc::UnboundedSender<StreamCenterEvent>,
}

impl RtmpPublishServer {
    pub fn new(
        stream_center_event_sender: mpsc::UnboundedSender<StreamCenterEvent>,
        config: &RtmpPublishServerConfig,
    ) -> Self {
        Self {
            config: config.clone(),
            stream_center_event_sender,
        }
    }

    #[instrument]
    pub async fn run(&mut self) -> RtmpPublishServerResult<()> {
        let listener = tokio::net::TcpListener::bind(("0.0.0.0", self.config.port)).await?;
        loop {
            let (tcp_stream, addr) = listener.accept().await?;
            tracing::info!("{}", addr);
            let mut session = RtmpPublishSession::new(
                tcp_stream,
                self.stream_center_event_sender.clone(),
                RtmpSessionConfig {
                    chunk_size: self.config.chunk_size,
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
