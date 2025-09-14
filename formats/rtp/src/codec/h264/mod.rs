pub mod aggregation;
pub mod errors;
pub mod fragmented;
pub mod packet;
pub mod paramters;
pub mod single_nalu;
pub(crate) mod util;
use crate::codec::h264::{aggregation::AggregatedHeader, fragmented::FUHeader};
use aggregation::AggregationNalUnits;
use byteorder::ReadBytesExt;
use errors::RtpH264Error;
use fragmented::{FragmentedUnit, FuIndicator};
use single_nalu::SingleNalUnit;
use std::io;
use utils::traits::{
    dynamic_sized_packet::DynamicSizedPacket,
    reader::{ReadFrom, ReadRemainingFrom},
    writer::WriteTo,
};

#[derive(Debug, Clone, Copy)]
pub enum PayloadStructureType {
    SingleNALUPacket(u8),
    AggregationPacket(AggregatedHeader),
    FragmentationUnit(FuIndicator),
}

impl From<PayloadStructureType> for u8 {
    fn from(value: PayloadStructureType) -> Self {
        match value {
            PayloadStructureType::SingleNALUPacket(v) => v,
            PayloadStructureType::AggregationPacket(v) => v.into(),
            PayloadStructureType::FragmentationUnit(v) => v.into(),
        }
    }
}

impl TryFrom<u8> for PayloadStructureType {
    type Error = RtpH264Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value & 0x1F {
            v if (1..=23).contains(&v) => Ok(Self::SingleNALUPacket(value)),
            v if (24..=27).contains(&v) => {
                Ok(Self::AggregationPacket(AggregatedHeader::try_from(value)?))
            }
            v if v == 28 || v == 29 => Ok(Self::FragmentationUnit(value.try_into().unwrap())),
            _ => Err(RtpH264Error::InvalidH264PacketType(value)),
        }
    }
}

#[derive(Debug)]
pub enum RtpH264NalUnit {
    SingleNalu(SingleNalUnit),
    Aggregated(AggregationNalUnits),
    Fragmented(FragmentedUnit),
}

impl RtpH264NalUnit {
    pub fn first_byte(&self) -> u8 {
        match self {
            Self::SingleNalu(nalu) => nalu.0.header.into(),
            Self::Aggregated(nalu) => nalu.header.into(),
            Self::Fragmented(nalu) => match nalu {
                FragmentedUnit::FuA(packet) => packet.indicator.into(),
                FragmentedUnit::FuB(packet) => packet.indicator.into(),
            },
        }
    }

    pub fn is_fragmented(&self) -> bool {
        matches!(self, Self::Fragmented(_))
    }

    pub fn fu_header(&self) -> Option<FUHeader> {
        match self {
            Self::Fragmented(f) => match f {
                FragmentedUnit::FuA(fa) => Some(fa.fu_header),
                FragmentedUnit::FuB(fb) => Some(fb.fu_header),
            },
            _ => None,
        }
    }
}

impl<R: io::Read> ReadFrom<R> for RtpH264NalUnit {
    type Error = RtpH264Error;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let first_byte = reader.read_u8()?;
        let payload_structure: PayloadStructureType = first_byte.try_into()?;
        let packet: RtpH264NalUnit = match payload_structure {
            PayloadStructureType::SingleNALUPacket(header) => {
                RtpH264NalUnit::SingleNalu(SingleNalUnit::read_remaining_from(header, reader)?)
            }
            PayloadStructureType::AggregationPacket(header) => RtpH264NalUnit::Aggregated(
                AggregationNalUnits::read_remaining_from(header, reader)?,
            ),
            PayloadStructureType::FragmentationUnit(header) => {
                RtpH264NalUnit::Fragmented(FragmentedUnit::read_remaining_from(header, reader)?)
            }
        };

        Ok(packet)
    }
}

impl<W: io::Write> WriteTo<W> for RtpH264NalUnit {
    type Error = RtpH264Error;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        match &self {
            RtpH264NalUnit::SingleNalu(packet) => packet.write_to(writer),
            RtpH264NalUnit::Aggregated(packet) => packet.write_to(writer),
            RtpH264NalUnit::Fragmented(packet) => packet.write_to(writer),
        }
    }
}

impl DynamicSizedPacket for RtpH264NalUnit {
    fn get_packet_bytes_count(&self) -> usize {
        match &self {
            RtpH264NalUnit::SingleNalu(packet) => packet.get_packet_bytes_count(),
            RtpH264NalUnit::Aggregated(packet) => packet.get_packet_bytes_count(),
            RtpH264NalUnit::Fragmented(packet) => packet.get_packet_bytes_count(),
        }
    }
}
