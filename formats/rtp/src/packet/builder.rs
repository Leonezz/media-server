use tokio_util::bytes::BytesMut;
use utils::system::time::get_timestamp_ms;

use crate::errors::{RtpError, RtpResult};

use super::{
    RtpPacket,
    header::{RtpHeader, RtpHeaderExtension},
};

#[derive(Debug, Default)]
pub struct RtpPacketBuilder {
    header: RtpHeader,
    payload: BytesMut,
}

impl RtpPacketBuilder {
    pub fn version(mut self, version: u8) -> Self {
        self.header.version = version;
        self
    }

    pub fn csrc(mut self, csrc: u32) -> RtpResult<Self> {
        if self.header.csrc_list.len() > 30 {
            return Err(RtpError::TooManyCSRC);
        }
        self.header.csrc_list.push(csrc);
        self.header.csrc_count = self.header.csrc_list.len() as u8;
        Ok(self)
    }

    pub fn marker(mut self, marker: bool) -> Self {
        self.header.marker = marker;
        self
    }

    pub fn payload_type(mut self, payload_type: u8) -> Self {
        self.header.payload_type = payload_type;
        self
    }

    pub fn sequence_number(mut self, number: u16) -> Self {
        self.header.sequence_number = number;
        self
    }

    pub fn timestamp(mut self, timestamp: u32) -> Self {
        self.header.timestamp = timestamp;
        self
    }

    pub fn timestamp_now(self) -> Self {
        self.timestamp(get_timestamp_ms().unwrap_or(0) as u32)
    }

    pub fn ssrc(mut self, ssrc: u32) -> Self {
        self.header.ssrc = ssrc;
        self
    }

    pub fn extension(mut self, extension: RtpHeaderExtension) -> Self {
        self.header.header_extension = Some(extension);
        self
    }

    pub fn payload(mut self, payload: &[u8]) -> Self {
        self.payload.extend_from_slice(payload);
        self
    }

    pub fn build(self) -> RtpPacket {
        RtpPacket {
            header: self.header,
            payload: self.payload.freeze(),
        }
    }
}
