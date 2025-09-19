use std::fmt;

use utils::traits::dynamic_sized_packet::DynamicSizedPacket;

use crate::{
    rtcp_context::RtpSessionObserver, rtcp_observer::RtcpObserver, rtp_observer::RtpObserver,
};

#[derive(Default)]
pub struct RtpSessionSimpleStatistics {
    total_rtp_packets_received: u64,
    total_rtp_packets_sent: u64,
    total_rtp_bytes_received: u64,
    total_rtp_bytes_sent: u64,
    total_rtcp_compound_packets_received: u64,
    total_rtcp_packets_received: u64,
    total_rtcp_compound_packets_sent: u64,
    total_rtcp_packets_sent: u64,
    total_rtcp_bytes_received: u64,
    total_rtcp_bytes_sent: u64,
}

impl RtpSessionObserver for RtpSessionSimpleStatistics {}

impl RtpSessionSimpleStatistics {
    pub fn new() -> Self {
        Default::default()
    }
}

impl fmt::Debug for RtpSessionSimpleStatistics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "total rtp packets received: {}",
            self.total_rtp_packets_received
        )?;
        writeln!(f, "total rtp packets sent: {}", self.total_rtp_packets_sent)?;
        writeln!(
            f,
            "total rtp bytes received: {}",
            self.total_rtp_bytes_received
        )?;
        writeln!(f, "total rtp bytes sent: {}", self.total_rtp_bytes_sent)?;
        writeln!(
            f,
            "total rtcp compound packets received: {}",
            self.total_rtcp_compound_packets_received
        )?;
        writeln!(
            f,
            "total rtcp compound packets sent: {}",
            self.total_rtcp_compound_packets_sent
        )?;
        writeln!(
            f,
            "total rtcp packets received: {}",
            self.total_rtcp_packets_received
        )?;
        writeln!(
            f,
            "total rtcp packets sent: {}",
            self.total_rtcp_packets_sent
        )?;
        writeln!(
            f,
            "total rtcp bytes received: {}",
            self.total_rtcp_bytes_received
        )?;
        writeln!(f, "total rtcp bytes sent: {}", self.total_rtcp_bytes_sent)?;
        Ok(())
    }
}

impl Drop for RtpSessionSimpleStatistics {
    fn drop(&mut self) {
        tracing::info!("rtp session simple statistics: \n{:?}", self);
    }
}

impl RtcpObserver for RtpSessionSimpleStatistics {
    fn on_rtcp_compound_packet_received(
        &mut self,
        packet: &rtp_formats::rtcp::compound_packet::RtcpCompoundPacket,
        _timestamp: std::time::SystemTime,
    ) {
        self.total_rtcp_compound_packets_received += 1;
        self.total_rtcp_packets_received += packet.packets().len() as u64;
        self.total_rtcp_bytes_received += packet.packets().iter().fold(0_u64, |prev, item| {
            prev + item.get_packet_bytes_count() as u64
        })
    }

    fn on_rtcp_compound_packet_sent(
        &mut self,
        packet: &rtp_formats::rtcp::compound_packet::RtcpCompoundPacket,
        _timestamp: std::time::SystemTime,
    ) {
        self.total_rtcp_compound_packets_sent += 1;
        self.total_rtcp_packets_sent += packet.packets().len() as u64;
        self.total_rtcp_bytes_sent += packet.packets().iter().fold(0_u64, |prev, item| {
            prev + item.get_packet_bytes_count() as u64
        })
    }
}

impl RtpObserver for RtpSessionSimpleStatistics {
    fn on_rtp_packet_received(
        &mut self,
        packet: &rtp_formats::packet::RtpTrivialPacket,
        _timestamp: std::time::SystemTime,
    ) {
        self.total_rtp_packets_received += 1;
        self.total_rtp_bytes_received += packet.get_packet_bytes_count() as u64;
    }

    fn on_rtp_packet_sent(
        &mut self,
        packet: &rtp_formats::packet::RtpTrivialPacket,
        _timestamp: std::time::SystemTime,
    ) {
        self.total_rtp_packets_sent += 1;
        self.total_rtp_bytes_sent += packet.get_packet_bytes_count() as u64;
    }
}
