use std::net::IpAddr;

use stream_center::events::StreamCenterEvent;
use tokio::sync::mpsc;

use super::{errors::RtmpServerResult, session::RtmpSession};

#[derive(Debug)]
pub struct RtmpServer {
    address: IpAddr,
    port: u16,
    chunk_size: u32,
    write_timeout_ms: u64,
    read_timeout_ms: u64,
    stream_center_event_sender: mpsc::UnboundedSender<StreamCenterEvent>,
}

impl RtmpServer {
    pub fn new(
        address: IpAddr,
        port: u16,
        chunk_size: u32,
        write_timeout_ms: u64,
        read_timeout_ms: u64,
        stream_center_event_sender: mpsc::UnboundedSender<StreamCenterEvent>,
    ) -> Self {
        Self {
            address,
            port,
            chunk_size,
            write_timeout_ms,
            read_timeout_ms,
            stream_center_event_sender,
        }
    }

    pub async fn run(&mut self) -> RtmpServerResult<()> {
        tracing::info!(
            "rtmp server is running at: tcp://{}:{}",
            self.address,
            self.port
        );
        let listener = tokio::net::TcpListener::bind((self.address, self.port)).await?;
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
                self.chunk_size,
                self.write_timeout_ms,
                self.read_timeout_ms,
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
                let _ = session.clean_up().await;
            });
        }
    }
}
