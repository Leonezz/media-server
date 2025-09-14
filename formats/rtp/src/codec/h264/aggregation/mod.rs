use super::errors::RtpH264Error;
use byteorder::WriteBytesExt;
use mtap::{Mtap16Format, Mtap24Format};
use stap::{StapAFormat, StapBFormat};
use std::io;
use utils::traits::{
    dynamic_sized_packet::DynamicSizedPacket,
    fixed_packet::FixedPacket,
    reader::{ReadFrom, ReadRemainingFrom},
    writer::WriteTo,
};

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

#[derive(Debug, Clone, Copy)]
pub struct AggregatedHeader {
    pub forbidden_zero_bit: bool,
    pub nal_ref_idc: u8,                       // 2 bits
    pub aggregate_type: AggregationPacketType, // 5 bits
}

impl FixedPacket for AggregatedHeader {
    fn bytes_count() -> usize {
        1
    }
}

impl TryFrom<u8> for AggregatedHeader {
    type Error = RtpH264Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let forbidden_zero_bit = ((value >> 7) & 0b1) == 0b1;
        assert!(!forbidden_zero_bit);
        let nal_ref_idc = (value >> 5) & 0b11;
        let agg_type = value & 0b1_1111;
        Ok(Self {
            forbidden_zero_bit,
            nal_ref_idc,
            aggregate_type: agg_type.try_into()?,
        })
    }
}

impl From<AggregatedHeader> for u8 {
    fn from(value: AggregatedHeader) -> Self {
        assert!(!value.forbidden_zero_bit);
        (value.nal_ref_idc << 5) | (Into::<u8>::into(value.aggregate_type))
    }
}

#[derive(Debug)]
pub enum AggregatedPayload {
    StapA(StapAFormat),
    StapB(StapBFormat),
    Mtap16(Mtap16Format),
    Mtap24(Mtap24Format),
}

#[derive(Debug)]
pub struct AggregationNalUnits {
    pub header: AggregatedHeader,
    pub payload: AggregatedPayload,
}

impl<R: io::Read> ReadRemainingFrom<AggregatedHeader, R> for AggregationNalUnits {
    type Error = RtpH264Error;
    fn read_remaining_from(header: AggregatedHeader, reader: &mut R) -> Result<Self, Self::Error> {
        match header.aggregate_type {
            AggregationPacketType::STAPA => Ok(Self {
                header,
                payload: AggregatedPayload::StapA(StapAFormat::read_from(reader)?),
            }),
            AggregationPacketType::STAPB => Ok(Self {
                header,
                payload: AggregatedPayload::StapB(StapBFormat::read_from(reader)?),
            }),
            AggregationPacketType::MTAP16 => Ok(Self {
                header,
                payload: AggregatedPayload::Mtap16(Mtap16Format::read_from(reader)?),
            }),
            AggregationPacketType::MTAP24 => Ok(Self {
                header,
                payload: AggregatedPayload::Mtap24(Mtap24Format::read_from(reader)?),
            }),
        }
    }
}

impl<W: io::Write> WriteTo<W> for AggregationNalUnits {
    type Error = RtpH264Error;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u8(self.header.into())?;
        match &self.payload {
            AggregatedPayload::StapA(packet) => packet.write_to(writer),
            AggregatedPayload::StapB(packet) => packet.write_to(writer),
            AggregatedPayload::Mtap16(packet) => packet.write_to(writer),
            AggregatedPayload::Mtap24(packet) => packet.write_to(writer),
        }
    }
}

impl DynamicSizedPacket for AggregationNalUnits {
    fn get_packet_bytes_count(&self) -> usize {
        AggregatedHeader::bytes_count()
            + match &self.payload {
                AggregatedPayload::StapA(packet) => packet.get_packet_bytes_count(),
                AggregatedPayload::StapB(packet) => packet.get_packet_bytes_count(),
                AggregatedPayload::Mtap16(packet) => packet.get_packet_bytes_count(),
                AggregatedPayload::Mtap24(packet) => packet.get_packet_bytes_count(),
            }
    }
}
