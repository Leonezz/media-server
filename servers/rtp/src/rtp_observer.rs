use std::time::SystemTime;

use rtp_formats::packet::RtpTrivialPacket;

pub trait RtpObserver: Send {
    fn on_rtp_packet_received(&mut self, packet: &RtpTrivialPacket, timestamp: SystemTime);
    fn on_rtp_packet_sent(&mut self, packet: &RtpTrivialPacket, timestamp: SystemTime);
}
