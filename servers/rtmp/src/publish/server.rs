use super::{
    config::RtmpPublishServerConfig, errors::RtmpPublishServerResult, session::RtmpPublishSession,
};

pub struct RtmpPublishServer {
    config: RtmpPublishServerConfig,
}

impl RtmpPublishServer {
    pub fn new(config: &RtmpPublishServerConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    pub async fn run(&mut self) -> RtmpPublishServerResult<()> {
        let listener = tokio::net::TcpListener::bind(("0.0.0.0", self.config.port)).await?;
        loop {
            let (tcp_stream, _) = listener.accept().await?;
            let mut session = RtmpPublishSession::new(tcp_stream);
            tokio::spawn(async move {
                match session.run().await {
                    Ok(()) => {
                        tracing::info!("successfully closed");
                    }
                    Err(err) => {
                        tracing::error!("{:?}", err);
                    }
                };
            });
        }
    }
}
