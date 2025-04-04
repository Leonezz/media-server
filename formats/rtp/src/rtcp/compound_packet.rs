use std::{
    cmp::Ordering,
    io::{self, Read},
};

use tokio_util::bytes::Buf;
use utils::traits::{
    dynamic_sized_packet::DynamicSizedPacket,
    fixed_packet::FixedPacket,
    reader::{TryReadFrom, TryReadRemainingFrom},
    writer::WriteTo,
};

use crate::errors::{RtpError, RtpResult};

use super::{
    RtcpPacket, RtcpPacketSizeTrait, RtcpPacketTrait, common_header::RtcpCommonHeader,
    payload_types::RtcpPayloadType, rtp_get_padding_size,
};

#[derive(Debug, Default, Clone)]
pub struct RtcpCompoundPacket {
    packets: Vec<RtcpPacket>,
}

impl RtcpPacketSizeTrait for RtcpCompoundPacket {
    fn get_packet_bytes_count_without_padding(&self) -> usize {
        self.packets()
            .iter()
            .fold(0, |sum, v| sum + v.get_packet_bytes_count_without_padding())
    }
    fn get_header(&self) -> super::common_header::RtcpCommonHeader {
        Default::default()
    }
}

impl DynamicSizedPacket for RtcpCompoundPacket {
    fn get_packet_bytes_count(&self) -> usize {
        let raw_size = self.get_packet_bytes_count_without_padding();
        raw_size + rtp_get_padding_size(raw_size)
    }
}

impl<R: AsRef<[u8]>> TryReadFrom<R> for RtcpCompoundPacket {
    type Error = RtpError;
    fn try_read_from(reader: &mut std::io::Cursor<R>) -> Result<Option<Self>, Self::Error> {
        let mut packets = vec![];
        loop {
            if reader.remaining() < RtcpCommonHeader::bytes_count() {
                return Ok(None);
            }
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
        let result = Self { packets };
        result.validate()?;
        Ok(Some(result))
    }
}

impl<W: io::Write> WriteTo<W> for RtcpCompoundPacket {
    type Error = RtpError;
    fn write_to(&self, mut writer: W) -> Result<(), Self::Error> {
        self.validate()?;
        self.packets()
            .iter()
            .try_for_each(|packet| packet.write_to(writer.by_ref()))?;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct RtcpCompoundPacketBuilder(RtcpCompoundPacket);

impl RtcpCompoundPacketBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn packet(mut self, packet: RtcpPacket) -> Self {
        self.0.packets_mut().push(packet);
        self
    }

    pub fn packets(mut self, mut packets: Vec<RtcpPacket>) -> Self {
        self.0.packets_mut().append(&mut packets);
        self
    }

    pub fn build(mut self) -> RtpResult<RtcpCompoundPacket> {
        self.0.sort();
        self.0.validate().map(|()| self.0)
    }
}

impl RtcpCompoundPacket {
    pub fn builder() -> RtcpCompoundPacketBuilder {
        RtcpCompoundPacketBuilder::new()
    }

    pub fn packets(&self) -> &Vec<RtcpPacket> {
        &self.packets
    }

    pub fn packets_mut(&mut self) -> &mut Vec<RtcpPacket> {
        &mut self.packets
    }

    pub fn sort(&mut self) {
        self.packets_mut().sort_by(|l, r| {
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
            Ordering::Equal
        });
    }

    pub fn validate(&self) -> RtpResult<()> {
        if self.packets().is_empty() {
            return Err(RtpError::EmptyRtcpCompoundPacket);
        }

        {
            let payload_type = self.packets()[0].payload_type();
            if payload_type != RtcpPayloadType::SenderReport
                && payload_type != RtcpPayloadType::ReceiverReport
            {
                return Err(RtpError::BadFirstPacketInRtcpCompound);
            }
        }

        for packet in self.packets()[1..].iter() {
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
