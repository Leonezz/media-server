use std::io;

use mtap::{Mtap16Packet, Mtap24Packet};
use stap::{StapAPacket, StapBPacket};
use utils::traits::{reader::ReadRemainingFrom, writer::WriteTo};

use crate::errors::RtpError;

pub mod mtap;
pub mod stap;

///! @see: RFC 6184 5.2. Payload Structures
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum AggregationPacketType {
    STAPA = 24, // Single-Time Aggregation Packet type A
    STAPB = 25, // Single-Time Aggregation Packet type B
    MTAP16 = 26, // Multi-Time Aggregation Packet (MTAP) with 16-bit offset
    MTAP24 = 27, // Multi-Time Aggregation Packet (MTAP) with 24-bit offset
}

impl Into<u8> for AggregationPacketType {
    fn into(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for AggregationPacketType {
    type Error = RtpError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            24 => Ok(Self::STAPA),
            25 => Ok(Self::STAPB),
            26 => Ok(Self::MTAP16),
            27 => Ok(Self::MTAP24),
            v => Err(RtpError::InvalidH264PacketType(v)),
        }
    }
}

#[derive(Debug)]
pub enum AggregationPacket {
    StapA(StapAPacket),
    StapB(StapBPacket),
    Mtap16(Mtap16Packet),
    Mtap24(Mtap24Packet),
}

impl<R: io::Read> ReadRemainingFrom<AggregationPacketType, R> for AggregationPacket {
    type Error = RtpError;
    fn read_remaining_from(header: AggregationPacketType, reader: R) -> Result<Self, Self::Error> {
        match header {
            AggregationPacketType::STAPA => Ok(Self::StapA(StapAPacket::read_remaining_from(
                header.into(),
                reader,
            )?)),
            AggregationPacketType::STAPB => Ok(Self::StapB(StapBPacket::read_remaining_from(
                header.into(),
                reader,
            )?)),
            AggregationPacketType::MTAP16 => Ok(Self::Mtap16(Mtap16Packet::read_remaining_from(
                header.into(),
                reader,
            )?)),
            AggregationPacketType::MTAP24 => Ok(Self::Mtap24(Mtap24Packet::read_remaining_from(
                header.into(),
                reader,
            )?)),
        }
    }
}

impl<W: io::Write> WriteTo<W> for AggregationPacket {
    type Error = RtpError;
    fn write_to(&self, writer: W) -> Result<(), Self::Error> {
        match self {
            Self::StapA(packet) => packet.write_to(writer),
            Self::StapB(packet) => packet.write_to(writer),
            Self::Mtap16(packet) => packet.write_to(writer),
            Self::Mtap24(packet) => packet.write_to(writer),
        }
    }
}
