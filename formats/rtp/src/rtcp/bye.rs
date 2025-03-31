use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use num::ToPrimitive;
use std::io;
use utils::traits::{
    dynamic_sized_packet::DynamicSizedPacket, fixed_packet::FixedPacket, reader::ReadRemainingFrom,
    writer::WriteTo,
};

use crate::{
    errors::{RtpError, RtpResult},
    util::padding::{rtp_get_padding_size, rtp_make_padding_bytes, rtp_need_padding},
};

use super::{RtcpPacketSizeTrait, common_header::RtcpCommonHeader, payload_types::RtcpPayloadType};

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

#[derive(Debug, Default, Clone)]
pub struct RtcpByePacket {
    pub header: RtcpCommonHeader,
    pub ssrc_list: Vec<u32>,
    pub leave_reason: Option<String>,
}

impl RtcpByePacket {
    pub fn builder() -> RtcpByePacketBuilder {
        RtcpByePacketBuilder::new()
    }
}

impl DynamicSizedPacket for RtcpByePacket {
    fn get_packet_bytes_count(&self) -> usize {
        let raw_bytes_count = self.get_packet_bytes_count_without_padding();
        raw_bytes_count + rtp_get_padding_size(raw_bytes_count)
    }
}

impl RtcpPacketSizeTrait for RtcpByePacket {
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

        let length = reader.read_u8()?;
        let mut buffer = vec![0_u8; length as usize];
        reader.read_exact(&mut buffer)?;
        let leave_reason = if !buffer.is_empty() {
            Some(String::from_utf8(buffer).unwrap())
        } else {
            None
        };

        let padding_bytes = rtp_get_padding_size(length as usize);
        if padding_bytes != 0 {
            reader.read_exact(&mut vec![0_u8; padding_bytes])?;
        }

        Ok(Self {
            header,
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
            let length = buffer.len();
            writer.write_u8(length.to_u8().unwrap())?;
            writer.write_all(buffer.as_bytes())?;
        }

        if let Some(buffer) = rtp_make_padding_bytes(self.get_packet_bytes_count_without_padding())
        {
            writer.write_all(&buffer)?;
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct RtcpByePacketBuilder(RtcpByePacket);

impl RtcpByePacketBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn ssrc(mut self, ssrc: u32) -> Self {
        self.0.ssrc_list.push(ssrc);
        self
    }

    pub fn ssrcs(mut self, mut ssrcs: Vec<u32>) -> Self {
        self.0.ssrc_list.append(&mut ssrcs);
        self
    }

    pub fn reason(mut self, reason: String) -> Self {
        self.0.leave_reason = Some(reason);
        self
    }

    pub fn build(self) -> RtpResult<RtcpByePacket> {
        if self.0.ssrc_list.len() > 31 {
            return Err(RtpError::TooManyCSRC);
        }

        if let Some(reason) = &self.0.leave_reason {
            if reason.len() > 255 {
                return Err(RtpError::ByeReasonTooLarge(reason.clone()));
            }
        }

        Ok(self.0)
    }
}
