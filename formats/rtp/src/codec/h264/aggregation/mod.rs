use std::io;

use mtap::{Mtap16Format, Mtap24Format};
use stap::{StapAFormat, StapBFormat};
use utils::traits::{
    dynamic_sized_packet::DynamicSizedPacket, reader::ReadRemainingFrom, writer::WriteTo,
};

use super::errors::RtpH264Error;

pub mod mtap;
pub mod stap;

// @see: RFC 6184 5.2. Payload Structures
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum AggregationPacketType {
    STAPA = 24,  // Single-Time Aggregation Packet type A
    STAPB = 25,  // Single-Time Aggregation Packet type B
    MTAP16 = 26, // Multi-Time Aggregation Packet (MTAP) with 16-bit offset
    MTAP24 = 27, // Multi-Time Aggregation Packet (MTAP) with 24-bit offset
}

impl From<AggregationPacketType> for u8 {
    fn from(value: AggregationPacketType) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for AggregationPacketType {
    type Error = RtpH264Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value & 0x1F {
            24 => Ok(Self::STAPA),
            25 => Ok(Self::STAPB),
            26 => Ok(Self::MTAP16),
            27 => Ok(Self::MTAP24),
            v => Err(RtpH264Error::InvalidH264PacketType(v)),
        }
    }
}

#[derive(Debug)]
pub enum AggregationNalUnits {
    StapA(StapAFormat),
    StapB(StapBFormat),
    Mtap16(Mtap16Format),
    Mtap24(Mtap24Format),
}

impl<R: io::Read> ReadRemainingFrom<AggregationPacketType, R> for AggregationNalUnits {
    type Error = RtpH264Error;
    fn read_remaining_from(
        header: AggregationPacketType,
        reader: &mut R,
    ) -> Result<Self, Self::Error> {
        match header {
            AggregationPacketType::STAPA => Ok(Self::StapA(StapAFormat::read_remaining_from(
                header.into(),
                reader,
            )?)),
            AggregationPacketType::STAPB => Ok(Self::StapB(StapBFormat::read_remaining_from(
                header.into(),
                reader,
            )?)),
            AggregationPacketType::MTAP16 => Ok(Self::Mtap16(Mtap16Format::read_remaining_from(
                header.into(),
                reader,
            )?)),
            AggregationPacketType::MTAP24 => Ok(Self::Mtap24(Mtap24Format::read_remaining_from(
                header.into(),
                reader,
            )?)),
        }
    }
}

impl<W: io::Write> WriteTo<W> for AggregationNalUnits {
    type Error = RtpH264Error;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        match self {
            Self::StapA(packet) => packet.write_to(writer),
            Self::StapB(packet) => packet.write_to(writer),
            Self::Mtap16(packet) => packet.write_to(writer),
            Self::Mtap24(packet) => packet.write_to(writer),
        }
    }
}

impl DynamicSizedPacket for AggregationNalUnits {
    fn get_packet_bytes_count(&self) -> usize {
        match self {
            Self::StapA(packet) => packet.get_packet_bytes_count(),
            Self::StapB(packet) => packet.get_packet_bytes_count(),
            Self::Mtap16(packet) => packet.get_packet_bytes_count(),
            Self::Mtap24(packet) => packet.get_packet_bytes_count(),
        }
    }
}
