use tokio_util::bytes::Bytes;

use super::RtspInterleavedPacket;

#[derive(Debug, Default)]
pub struct RtspInterleavedPacketBuilder {
    channel: u8,
    payload: Vec<u8>,
}

impl RtspInterleavedPacketBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn channel(mut self, channel: u8) -> Self {
        self.channel = channel;
        self
    }

    pub fn payload(mut self, payload: &[u8]) -> Self {
        self.payload.extend_from_slice(payload);
        self
    }

    pub fn build(self) -> RtspInterleavedPacket {
        RtspInterleavedPacket {
            channel_id: self.channel,
            payload: Bytes::from(self.payload),
        }
    }
}
