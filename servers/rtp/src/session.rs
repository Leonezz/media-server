use std::{pin::Pin, time::SystemTime};

use futures::{SinkExt, TryStreamExt};
use rtp_formats::{
    packet::{RtpTrivialPacket, framed::RtpTrivialPacketFramed},
    rtcp::framed::RtcpPacketFramed,
};
use tokio::sync::mpsc::{self, error::TryRecvError};
use tokio_util::codec::Framed;
use unified_io::UnifiedIO;

use crate::{
    errors::{RtpSessionError, RtpSessionResult},
    rtcp_context::RtcpContext,
    rtcp_observer::RtcpObserver,
    rtp_observer::RtpObserver,
};

#[derive(Debug)]
pub struct RtpSession {
    rtp_io: Framed<Pin<Box<dyn UnifiedIO>>, RtpTrivialPacketFramed>,
    rtcp_io: Framed<Pin<Box<dyn UnifiedIO>>, RtcpPacketFramed>,

    // receives rtp packets from application level, and send them through rtp_io internally
    rtp_rx: mpsc::UnboundedReceiver<RtpTrivialPacket>,
    // received rtp packets from rtp_io, and send them to application level through rtp_tx
    rtp_tx: mpsc::UnboundedSender<RtpTrivialPacket>,
    // rtp and rtcp observer
    rtcp_context: RtcpContext,
}

impl RtpSession {
    pub fn new(
        rtp_io: Pin<Box<dyn UnifiedIO>>,
        rtcp_io: Pin<Box<dyn UnifiedIO>>,
        session_bandwidth: u64,
        rtp_clockrate: u64,
        rtp_rx: mpsc::UnboundedReceiver<RtpTrivialPacket>,
        rtp_tx: mpsc::UnboundedSender<RtpTrivialPacket>,
    ) -> Self {
        Self {
            rtp_io: Framed::new(rtp_io, RtpTrivialPacketFramed),
            rtcp_io: Framed::new(rtcp_io, RtcpPacketFramed),
            rtp_rx,
            rtp_tx,
            rtcp_context: RtcpContext::new(session_bandwidth, rtp_clockrate),
        }
    }

    async fn run(&mut self) -> RtpSessionResult<()> {
        tracing::info!("rtp session is running");
        loop {
            self.try_send_rtp().await?;
            self.rtcp_session_tick();
            let now = SystemTime::now();
            if self.rtcp_context.timed_out(now) {
                let packet = self
                    .rtcp_context
                    .generate_rtcp_compound_packet(now, false, None)?;
                self.rtcp_io.send(packet.clone()).await?;
                self.rtcp_context.on_rtcp_compound_packet_sent(&packet, now);
            }

            self.try_receive_rtp().await?;
            self.try_receive_rtcp().await?;
        }
    }

    async fn try_receive_rtp(&mut self) -> RtpSessionResult<()> {
        match self.rtp_io.try_next().await {
            Ok(None) => Ok(()),
            Ok(Some(packet)) => {
                self.rtcp_context
                    .on_rtp_packet_received(&packet, SystemTime::now());
                self.rtp_tx
                    .send(packet)
                    .map_err(|_| RtpSessionError::RtpPacketChannelDisconnected)?;
                Ok(())
            }
            Err(err) => Err(RtpSessionError::RtpFormatError(err)),
        }
    }

    async fn try_receive_rtcp(&mut self) -> RtpSessionResult<()> {
        match self.rtcp_io.try_next().await {
            Ok(None) => Ok(()),
            Ok(Some(packet)) => {
                self.rtcp_context
                    .on_rtcp_compound_packet_received(&packet, SystemTime::now());
                Ok(())
            }
            Err(err) => Err(RtpSessionError::RtpFormatError(err)),
        }
    }

    async fn try_send_rtp(&mut self) -> RtpSessionResult<()> {
        match self.rtp_rx.try_recv() {
            Ok(packet) => {
                self.rtp_io.send(packet.clone()).await?;
                self.rtcp_context
                    .on_rtp_packet_sent(&packet, SystemTime::now());
                Ok(())
            }
            Err(TryRecvError::Empty) => Ok(()),
            Err(TryRecvError::Disconnected) => Err(RtpSessionError::RtpPacketChannelDisconnected),
        }
    }

    fn rtcp_session_tick(&mut self) {
        self.rtcp_context.check_timeout();
    }
}
