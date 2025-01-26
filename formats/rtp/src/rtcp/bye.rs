use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io;
use tokio_util::bytes::Bytes;
use utils::traits::{
    dynamic_sized_packet::DynamicSizedPacket, fixed_packet::FixedPacket, reader::ReadRemainingFrom,
    writer::WriteTo,
};

use crate::{
    errors::RtpError,
    util::padding::{rtp_get_padding_size, rtp_make_padding_bytes, rtp_need_padding},
};

use super::{RtcpPacketTrait, common_header::RtcpCommonHeader, payload_types::RtcpPayloadType};

// @see: RFC 3550 6.6 BYE: Goodbye RTCP Packet
///        0                   1                   2                   3
///        0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
///       +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///       |V=2|P|    SC   |   PT=BYE=203  |            length             |
///       +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///       |                             SSRC/CSRC                         |
///       +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///       :                               ...                             :
///       +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
/// (opt) |     length    |             reason for leaving              ...
///       +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///

#[derive(Debug)]
pub struct RtcpByePacket {
    _header: RtcpCommonHeader,
    ssrc_list: Vec<u32>,
    leave_reason: Option<Bytes>,
}

impl DynamicSizedPacket for RtcpByePacket {
    fn get_packet_bytes_count(&self) -> usize {
        let raw_bytes_count = self.get_packet_bytes_count_without_padding();
        raw_bytes_count + rtp_get_padding_size(raw_bytes_count)
    }
}

impl RtcpPacketTrait for RtcpByePacket {
    fn get_packet_bytes_count_without_padding(&self) -> usize {
        RtcpCommonHeader::bytes_count() // header
          + self.ssrc_list.len() * 4 // ssrc list
          + self.leave_reason.as_ref().map_or_else(|| 0, |v| v.len() + 1) // reason for leaving and reason length
    }
    fn get_header(&self) -> RtcpCommonHeader {
        let raw_size = self.get_packet_bytes_count_without_padding();
        RtcpCommonHeader {
            version: 2,
            padding: rtp_need_padding(raw_size),
            count: self.ssrc_list.len() as u8,
            payload_type: RtcpPayloadType::Bye,
            length: (self.get_packet_bytes_count() / 4 - 1) as u16,
        }
    }
}

impl<R: io::Read> ReadRemainingFrom<RtcpCommonHeader, R> for RtcpByePacket {
    type Error = RtpError;
    fn read_remaining_from(header: RtcpCommonHeader, mut reader: R) -> Result<Self, Self::Error> {
        if header.payload_type != RtcpPayloadType::Bye {
            return Err(RtpError::WrongPayloadType(format!(
                "expect bye payload type got {:?} instead",
                header.payload_type
            )));
        }
        let mut ssrc_list = Vec::with_capacity(header.count as usize);
        for _ in 0..header.count {
            ssrc_list.push(reader.read_u32::<BigEndian>()?);
        }

        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        let leave_reason = if !buffer.is_empty() {
            Some(Bytes::from(buffer))
        } else {
            None
        };

        Ok(Self {
            _header: header,
            ssrc_list,
            leave_reason,
        })
    }
}

impl<W: io::Write> WriteTo<W> for RtcpByePacket {
    type Error = RtpError;
    fn write_to(&self, mut writer: W) -> Result<(), Self::Error> {
        self.get_header().write_to(writer.by_ref())?;
        self.ssrc_list
            .iter()
            .try_for_each(|ssrc| writer.write_u32::<BigEndian>(*ssrc))?;

        if let Some(buffer) = &self.leave_reason {
            writer.write_all(buffer)?;
        }

        if let Some(buffer) = rtp_make_padding_bytes(self.get_packet_bytes_count_without_padding())
        {
            writer.write_all(&buffer)?;
        }
        Ok(())
    }
}
