use crate::errors::RtpError;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::{self, Cursor};
use tokio_util::bytes::Buf;
use utils::traits::{fixed_packet::FixedPacket, reader::TryReadFrom, writer::WriteTo};

use super::payload_types::RtcpPayloadType;

#[derive(Debug, Default, Clone)]
pub struct RtcpCommonHeader {
    pub version: u8,
    pub padding: bool,
    pub count: u8,
    pub payload_type: RtcpPayloadType,
    /// The length of this RTCP packet in 32-bit words minus one,
    /// including the header and any padding.
    /// (The offset of one makes zero a valid length and avoids
    /// a possible infinite loop in scanning a compound RTCP packet,
    /// while counting 32-bit words avoids a validity check for a multiple of 4.)
    pub length: u16,
}

impl FixedPacket for RtcpCommonHeader {
    #[inline]
    fn bytes_count() -> usize {
        4
    }
}

impl<R: AsRef<[u8]>> TryReadFrom<R> for RtcpCommonHeader {
    type Error = RtpError;
    fn try_read_from(reader: &mut Cursor<R>) -> Result<Option<Self>, Self::Error> {
        if reader.remaining() < Self::bytes_count() {
            return Ok(None);
        }
        let word = reader.read_u32::<BigEndian>()?;
        Ok(Some(Self {
            version: ((word >> 30) & 0b11) as u8,
            padding: ((word >> 29) & 0b1) == 0b1,
            count: ((word >> 24) & 0b1_1111) as u8,
            payload_type: (((word >> 16) & 0b1111_1111) as u8).try_into()?,
            length: (word & 0b1111_1111_1111_1111) as u16,
        }))
    }
}

impl<W: io::Write> WriteTo<W> for RtcpCommonHeader {
    type Error = RtpError;
    fn write_to(&self, mut writer: W) -> Result<(), Self::Error> {
        let word = ((self.version as u32) << 30)
            | ((self.padding as u32) << 29)
            | ((self.count as u32) << 24)
            | ((Into::<u8>::into(self.payload_type) as u32) << 16)
            | (self.length as u32);
        writer.write_u32::<BigEndian>(word)?;
        Ok(())
    }
}
