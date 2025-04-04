use futures::StreamExt;
use rtsp_formats::{RtspMessage, RtspMessageFramed};
use std::pin::Pin;
use tokio_util::codec::Framed;
use unified_io::UnifiedIO;

use crate::errors::{RtspServerError, RtspServerResult};

#[derive(Debug)]
pub struct RtspSession {
    io: Framed<Pin<Box<dyn UnifiedIO + Send>>, RtspMessageFramed>,
}

impl RtspSession {
    pub fn new(io: Pin<Box<dyn UnifiedIO + Send>>) -> Self {
        Self {
            io: Framed::new(io, RtspMessageFramed),
        }
    }

    pub async fn run(&mut self) -> RtspServerResult<()> {
        tracing::info!("rtsp session is running");
        loop {
            match self.io.next().await {
                Some(Ok(message)) => {
                    tracing::info!("Received RTSP message: {:?}", message);
                    // Process the message here
                }
                Some(Err(e)) => {
                    tracing::error!("Error receiving RTSP message: {:?}", e);
                    return Err(RtspServerError::RtspMessageError(e));
                }
                None => {
                    tracing::info!("RTSP session closed");
                    break;
                }
            }
        }
        Ok(())
    }
}
