use crate::{
    errors::{RtpSessionError, RtpSessionResult},
    rtcp_context::{RtcpContext, RtpSessionObserver},
    rtcp_observer::RtcpObserver,
    rtp_observer::RtpObserver,
};
use futures::{FutureExt, SinkExt, StreamExt, select};
use rtp_formats::{
    packet::{RtpTrivialPacket, framed::RtpTrivialPacketFramed},
    rtcp::{RtcpPacket, compound_packet::RtcpCompoundPacket, framed::RtcpPacketFramed},
};
use std::{
    io,
    pin::Pin,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::{
    RwLock,
    mpsc::{self, error::TryRecvError},
};
use unified_io::{UnifiedIO, UnifiyStreamed};

pub enum RtpSessionCommand {
    Stop,
    Start,
    Rtp(RtpTrivialPacket),
    Rtcp(RtcpPacket),
}

pub struct RtpSession {
    command_rx: Arc<RwLock<mpsc::Receiver<RtpSessionCommand>>>,
    // received rtp packets from rtp_io, and send them to application level through rtp_tx
    rtp_tx: Option<mpsc::Sender<RtpTrivialPacket>>,
    // rtp and rtcp observer
    rtcp_context: Arc<RwLock<RtcpContext>>,
}

impl RtpSession {
    pub fn new(
        ssrc: u32,
        cname: Option<String>,
        session_bandwidth: u64,
        rtp_clockrate: u64,
        command_rx: mpsc::Receiver<RtpSessionCommand>,
        rtp_tx: Option<mpsc::Sender<RtpTrivialPacket>>,
    ) -> Self {
        Self {
            command_rx: Arc::new(RwLock::new(command_rx)),
            rtp_tx,
            rtcp_context: Arc::new(RwLock::new(RtcpContext::new(
                session_bandwidth,
                rtp_clockrate,
                cname,
                ssrc,
            ))),
        }
    }

    pub async fn run(
        &mut self,
        send: bool,
        rtp_io: Pin<Box<dyn UnifiedIO>>,
        rtcp_io: Pin<Box<dyn UnifiedIO>>,
    ) -> RtpSessionResult<()> {
        let (rtp_sender, rtp_receiver) = mpsc::channel(1000);
        let (rtcp_sender, rtcp_receiver) = mpsc::channel(1000);
        select! {
            result = Self::run_rtp(send, rtp_io, self.rtcp_context.clone(), self.rtp_tx.clone(), rtp_receiver).fuse() => {
                if let Err(err) = &result {
                    tracing::error!("rtp thread got error: {}", err);
                }
                tracing::info!("rtp session is about to exit because rtp thread exited, {:?}", result);
                result
            }
            result = Self::run_rtcp(send, rtcp_io, self.rtcp_context.clone(), rtcp_receiver).fuse() => {
                if let Err(err) = &result {
                    tracing::error!("rtcp thread got error: {}", err);
                }
                tracing::info!("rtp session is about to exit because rtcp thread exited, {:?}", result);
                result
            }
            result = Self::run_command(self.command_rx.clone(), rtp_sender, rtcp_sender).fuse() => {
                if let Err(err) = &result && !matches!(err, RtpSessionError::GracefulExit) {
                    tracing::error!("command thread got error: {}", err);
                }
                tracing::info!("rtp session is about to exit because command thread exited, {:?}", result);
                result
            }

        }
    }

    async fn run_rtp(
        send: bool,
        rtp_io: Pin<Box<dyn UnifiedIO>>,
        rtcp_context: Arc<RwLock<RtcpContext>>,
        rtp_tx: Option<mpsc::Sender<RtpTrivialPacket>>,
        mut rtp_rx: mpsc::Receiver<RtpTrivialPacket>,
    ) -> RtpSessionResult<()> {
        let mut io = UnifiyStreamed::new(rtp_io, RtpTrivialPacketFramed);
        if send {
            loop {
                match rtp_rx.recv().await {
                    None => {
                        return Err(RtpSessionError::RtpPacketChannelDisconnected);
                    }
                    Some(packet) => {
                        rtcp_context
                            .write()
                            .await
                            .on_rtp_packet_sent(&packet, SystemTime::now());
                        io.send(packet).await?;
                    }
                }
            }
        } else if let Some(rtp_tx) = rtp_tx {
            loop {
                let packet = Self::receive_rtp(&mut io).await?;
                rtcp_context
                    .write()
                    .await
                    .on_rtp_packet_received(&packet, SystemTime::now());
                rtp_tx
                    .send_timeout(packet, Duration::from_secs(1))
                    .await
                    .map_err(|_| RtpSessionError::RtpPacketChannelDisconnected)?;

                // rtp packets might be received from commands channel
                match rtp_rx.try_recv() {
                    Err(TryRecvError::Disconnected) => {
                        return Err(RtpSessionError::RtpPacketChannelDisconnected);
                    }
                    Err(_) => {}
                    Ok(packet) => {
                        io.send(packet).await?;
                    }
                }
            }
        } else {
            Err(RtpSessionError::InvalidRtpSessionConfiguration(
                "rtp session is configured to receive rtp packets, but no rtp packet channel is provided"
                    .to_string(),
            ))
        }
    }

    async fn run_rtcp(
        send: bool,
        rtcp_io: Pin<Box<dyn UnifiedIO>>,
        rtcp_context: Arc<RwLock<RtcpContext>>,
        mut rtcp_rx: mpsc::Receiver<RtcpPacket>,
    ) -> RtpSessionResult<()> {
        let mut io = UnifiyStreamed::new(rtcp_io, RtcpPacketFramed);
        let mut rtcp_buffer = Vec::new();
        loop {
            if !send {
                let packet = Self::receive_rtcp(&mut io).await?;
                rtcp_context
                    .write()
                    .await
                    .on_rtcp_compound_packet_received(&packet, SystemTime::now());
            }
            match rtcp_rx.try_recv() {
                Err(TryRecvError::Disconnected) => {
                    return Err(RtpSessionError::RtcpPacketChannelDisconnected);
                }
                Err(_) => {}
                Ok(packet) => {
                    rtcp_buffer.push(packet);
                }
            }

            let now = SystemTime::now();
            {
                rtcp_context.write().await.check_timeout();
                if !rtcp_context.read().await.timed_out(now) {
                    continue;
                }
            }
            let packet = rtcp_context.read().await.generate_rtcp_compound_packet(
                now,
                false,
                None,
                rtcp_buffer.clone(),
            )?;
            rtcp_buffer.clear();
            io.send(packet.clone()).await?;
            rtcp_context
                .write()
                .await
                .on_rtcp_compound_packet_sent(&packet, now);
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    async fn run_command(
        command_rx: Arc<RwLock<mpsc::Receiver<RtpSessionCommand>>>,
        rtp_tx: mpsc::Sender<RtpTrivialPacket>,
        rtcp_tx: mpsc::Sender<RtcpPacket>,
    ) -> RtpSessionResult<()> {
        loop {
            match command_rx.write().await.recv().await {
                None => {
                    return Err(RtpSessionError::IoError(io::Error::new(
                        io::ErrorKind::ConnectionAborted,
                        "connect aborted by peer".to_string(),
                    )));
                }
                Some(command) => match command {
                    RtpSessionCommand::Start => {
                        tracing::info!("rtp session is starting");
                    }
                    RtpSessionCommand::Stop => {
                        tracing::info!("rtp session is grecefully stopping");
                        return Err(RtpSessionError::GracefulExit);
                    }
                    RtpSessionCommand::Rtp(packet) => rtp_tx
                        .send_timeout(packet, Duration::from_secs(1))
                        .await
                        .map_err(|err| {
                            RtpSessionError::SendRtpPacketToChannelFailed(format!("{}", err))
                        })?,
                    RtpSessionCommand::Rtcp(packet) => rtcp_tx
                        .send_timeout(packet, Duration::from_secs(1))
                        .await
                        .map_err(|err| {
                            RtpSessionError::SendRtcpPacketToChannelFailed(format!("{}", err))
                        })?,
                },
            }
        }
    }

    pub async fn with_observer(self, observer: Box<dyn RtpSessionObserver>) -> Self {
        self.rtcp_context.write().await.with_observer(observer);
        self
    }

    async fn receive_rtp(
        rtp_io: &mut UnifiyStreamed<RtpTrivialPacketFramed>,
    ) -> RtpSessionResult<RtpTrivialPacket> {
        let packet = rtp_io.next().await;
        match packet {
            None => Err(RtpSessionError::IoError(io::Error::new(
                io::ErrorKind::ConnectionAborted,
                "connect aborted by peer".to_string(),
            ))),
            Some(Err(err)) => Err(err.into()),
            Some(Ok(packet)) => Ok(packet),
        }
    }

    async fn receive_rtcp(
        rtcp_io: &mut UnifiyStreamed<RtcpPacketFramed>,
    ) -> RtpSessionResult<RtcpCompoundPacket> {
        let packet = rtcp_io.next().await;
        match packet {
            None => Err(RtpSessionError::IoError(io::Error::new(
                io::ErrorKind::ConnectionAborted,
                "connect aborted by peer".to_string(),
            ))),
            Some(Err(err)) => Err(err.into()),
            Some(Ok(packet)) => Ok(packet),
        }
    }
}
