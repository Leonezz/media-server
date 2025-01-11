use std::{
    cmp::Ordering,
    io::{self, Read},
};

use utils::traits::{
    dynamic_sized_packet::DynamicSizedPacket,
    reader::{TryReadFrom, TryReadRemainingFrom},
    writer::WriteTo,
};

use crate::errors::{RtpError, RtpResult};

use super::{
    RtcpPacket, RtcpPacketTrait, common_header::RtcpCommonHeader, payload_types::RtcpPayloadType,
    rtp_get_padding_size,
};

pub struct CompoundPacket(pub Vec<RtcpPacket>);

impl RtcpPacketTrait for CompoundPacket {
    fn get_packet_bytes_count_without_padding(&self) -> usize {
        self.0
            .iter()
            .fold(0, |sum, v| sum + v.get_packet_bytes_count_without_padding())
    }
    fn get_header(&self) -> super::common_header::RtcpCommonHeader {
        Default::default()
    }
}

impl DynamicSizedPacket for CompoundPacket {
    fn get_packet_bytes_count(&self) -> usize {
        let raw_size = self
            .0
            .iter()
            .fold(0, |sum, v| sum + v.get_packet_bytes_count());
        raw_size + rtp_get_padding_size(raw_size)
    }
}

impl<R: AsRef<[u8]>> TryReadFrom<R> for CompoundPacket {
    type Error = RtpError;
    fn try_read_from(reader: &mut std::io::Cursor<R>) -> Result<Option<Self>, Self::Error> {
        let mut packets = vec![];
        loop {
            let header = RtcpCommonHeader::try_read_from(reader.by_ref())?;
            if header.is_none() {
                break;
            }

            let packet = RtcpPacket::try_read_remaining_from(header.unwrap(), reader.by_ref())?;
            if packet.is_none() {
                return Ok(None);
            }
            packets.push(packet.unwrap());
        }
        let result = Self(packets);
        result.validate()?;
        Ok(Some(result))
    }
}

impl<W: io::Write> WriteTo<W> for CompoundPacket {
    type Error = RtpError;
    fn write_to(&self, mut writer: W) -> Result<(), Self::Error> {
        self.validate()?;
        self.0
            .iter()
            .try_for_each(|packet| packet.write_to(writer.by_ref()))?;
        Ok(())
    }
}

impl CompoundPacket {
    pub fn append(&mut self, packet: RtcpPacket) {
        self.0.push(packet);
        self.sort();
    }

    pub fn sort(&mut self) {
        self.0.sort_by(|l, r| {
            let l_type = l.payload_type();
            let r_type = r.payload_type();
            if l_type == RtcpPayloadType::SenderReport || l_type == RtcpPayloadType::ReceiverReport
            {
                return Ordering::Less;
            }
            if r_type == RtcpPayloadType::SenderReport || r_type == RtcpPayloadType::ReceiverReport
            {
                return Ordering::Greater;
            }
            if let RtcpPacket::SourceDescription(packet) = l {
                if packet.get_cname().is_some() {
                    return Ordering::Less;
                }
            }
            if let RtcpPacket::SourceDescription(packet) = r {
                if packet.get_cname().is_some() {
                    return Ordering::Greater;
                }
            }
            return Ordering::Equal;
        });
    }

    pub fn validate(&self) -> RtpResult<()> {
        if self.0.is_empty() {
            return Err(RtpError::EmptyRtcpCompoundPacket);
        }

        {
            let payload_type = self.0[0].payload_type();
            if payload_type != RtcpPayloadType::SenderReport
                && payload_type != RtcpPayloadType::ReceiverReport
            {
                return Err(RtpError::BadFirstPacketInRtcpCompound);
            }
        }

        for packet in self.0[1..].iter() {
            if packet.payload_type() == RtcpPayloadType::ReceiverReport {
                continue;
            }
            if let RtcpPacket::SourceDescription(pkt) = packet {
                if pkt.get_cname().is_none() {
                    return Err(RtpError::MissingCnameInRtcpCompound);
                } else {
                    return Ok(());
                }
            } else {
                return Err(RtpError::BadCnamePositionInRtcpCompound);
            }
        }
        Ok(())
    }
}
