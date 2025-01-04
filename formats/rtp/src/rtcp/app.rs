use std::io::{self};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use packet_traits::{
    dynamic_sized_packet::DynamicSizedPacket, fixed_packet::FixedPacket, reader::ReadRemainingFrom,
    writer::WriteTo,
};
use tokio_util::bytes::BytesMut;

use crate::errors::RtpError;

use super::{common_header::RtcpCommonHeader, payload_types::RtcpPayloadType};

///! @see: RFC 3550 6.7 APP: Application-Defined RTCP Packet
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

#[derive(Debug)]
pub struct RtcpAppPacket {
    header: RtcpCommonHeader,
    ssrc: u32,
    name: [u8; 4],
    payload: BytesMut,
}

impl DynamicSizedPacket for RtcpAppPacket {
    fn get_packet_bytes_count(&self) -> usize {
        RtcpCommonHeader::bytes_count() // header 
          + 4 // ssrc 
          + 4 // name 
          + self.payload.len() // app data
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
        let mut name = [0 as u8; 4];
        reader.read_exact(&mut name)?;
        let mut payload = Vec::new();
        reader.read_to_end(&mut payload)?;
        Ok(Self {
            header,
            ssrc,
            name,
            payload: BytesMut::from(&payload[..]),
        })
    }
}

impl<W: io::Write> WriteTo<W> for RtcpAppPacket {
    type Error = RtpError;
    fn write_to(&self, mut writer: W) -> Result<(), Self::Error> {
        self.header.write_to(writer.by_ref())?;
        writer.write_u32::<BigEndian>(self.ssrc)?;
        writer.write_all(&self.name)?;
        writer.write_all(&self.payload)?;
        Ok(())
    }
}
