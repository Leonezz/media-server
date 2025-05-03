use std::io::{self};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use tokio_util::bytes::Bytes;
use utils::traits::{
    dynamic_sized_packet::DynamicSizedPacket, fixed_packet::FixedPacket, reader::ReadRemainingFrom,
    writer::WriteTo,
};

use crate::{
    errors::RtpError,
    util::padding::{rtp_get_padding_size, rtp_make_padding_bytes},
};

use super::{RtcpPacketSizeTrait, common_header::RtcpCommonHeader, payload_types::RtcpPayloadType};

// @see: RFC 3550 6.7 APP: Application-Defined RTCP Packet
///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |V=2|P| subtype |  PT=APP=204   |              length           |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                           SSRC/CSRC                           |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                          name (ASCII)                         |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                   application-dependent data                ...
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

#[derive(Debug, Clone)]
pub struct RtcpAppPacket {
    pub header: RtcpCommonHeader,
    pub ssrc: u32,
    pub name: [u8; 4],
    pub payload: Bytes,
}

impl RtcpPacketSizeTrait for RtcpAppPacket {
    fn get_packet_bytes_count_without_padding(&self) -> usize {
        RtcpCommonHeader::bytes_count() // header
         + 4 // ssrc
         + 4 // name
         + self.payload.len()
    }
    fn get_header(&self) -> RtcpCommonHeader {
        todo!()
    }
}

impl DynamicSizedPacket for RtcpAppPacket {
    fn get_packet_bytes_count(&self) -> usize {
        let raw_bytes_count = self.get_packet_bytes_count_without_padding();
        raw_bytes_count + rtp_get_padding_size(raw_bytes_count)
    }
}

impl<R: io::Read> ReadRemainingFrom<RtcpCommonHeader, R> for RtcpAppPacket {
    type Error = RtpError;
    fn read_remaining_from(header: RtcpCommonHeader, mut reader: R) -> Result<Self, Self::Error> {
        if header.payload_type != RtcpPayloadType::App {
            return Err(RtpError::WrongPayloadType(format!(
                "expect app payload type got {:?} instead",
                header.payload_type
            )));
        }
        let ssrc = reader.read_u32::<BigEndian>()?;
        let mut name = [0_u8; 4];
        reader.read_exact(&mut name)?;
        let mut payload = Vec::new();
        reader.read_to_end(&mut payload)?;
        Ok(Self {
            header,
            ssrc,
            name,
            payload: Bytes::from(payload),
        })
    }
}

impl<W: io::Write> WriteTo<W> for RtcpAppPacket {
    type Error = RtpError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let raw_size = self.get_packet_bytes_count_without_padding();
        self.get_header().write_to(writer)?;
        writer.write_u32::<BigEndian>(self.ssrc)?;
        writer.write_all(&self.name)?;
        writer.write_all(&self.payload)?;
        if let Some(padding) = rtp_make_padding_bytes(raw_size) {
            writer.write_all(&padding)?;
        }
        Ok(())
    }
}
