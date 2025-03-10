use stream_center::events::StreamCenterEvent;
use tokio::sync::mpsc;

use crate::config::RtmpSessionConfig;

use super::{config::RtmpServerConfig, errors::RtmpServerResult, session::RtmpSession};

#[derive(Debug)]
pub struct RtmpServer {
    config: RtmpServerConfig,
    stream_center_event_sender: mpsc::UnboundedSender<StreamCenterEvent>,
}

impl RtmpServer {
    pub fn new(
        config: RtmpServerConfig,
        stream_center_event_sender: mpsc::UnboundedSender<StreamCenterEvent>,
    ) -> Self {
        Self {
            config: config,
            stream_center_event_sender,
        }
    }

    pub async fn run(&mut self) -> RtmpServerResult<()> {
        log::info!("rtmp server is running: {:?}", self.config);
        let listener =
            tokio::net::TcpListener::bind((self.config.address, self.config.port)).await?;
        loop {
            let (tcp_stream, addr) = listener.accept().await?;
            let peer_addr = tcp_stream.peer_addr();
            log::info!(
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
                        log::info!(
                            "rtmp session successfully closed, addr: {}, peer addr: {:?}",
                            addr,
                            peer_addr
                        );
                    }
                    Err(err) => {
                        log::error!("{:?}", err);
                    }
                };
                session.log_stats().await;
                let _ = session.clean_up().await;
            });
        }
    }
}
