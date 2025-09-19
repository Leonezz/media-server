use std::time::SystemTime;

use rtp_formats::rtcp::compound_packet::RtcpCompoundPacket;

pub trait RtcpObserver: Send {
    fn on_rtcp_compound_packet_received(
        &mut self,
        packet: &RtcpCompoundPacket,
        timestamp: SystemTime,
    );
    fn on_rtcp_compound_packet_sent(&mut self, packet: &RtcpCompoundPacket, timestamp: SystemTime);
}
