use tokio_util::bytes::BytesMut;

use crate::header::RtpHeader;

use super::RtpTrivialPacket;

#[derive(Debug, Default)]
pub struct RtpTrivialPacketBuilder {
    header: RtpHeader,
    payload: BytesMut,
}

impl RtpTrivialPacketBuilder {
    pub fn header(mut self, header: RtpHeader) -> Self {
        self.header = header;
        self
    }
    pub fn version(mut self, version: u8) -> Self {
        self.header.version = version;
        self
    }

    pub fn payload(mut self, payload: &[u8]) -> Self {
        self.payload.extend_from_slice(payload);
        self
    }

    pub fn build(self) -> RtpTrivialPacket {
        RtpTrivialPacket::new(self.header, self.payload.freeze())
    }
}
