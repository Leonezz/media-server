pub mod aggregation;
pub mod fragmented;
pub mod packet;
pub mod single_nalu;
pub(crate) mod util;
use std::io;

use aggregation::{AggregationNalUnits, AggregationPacketType};
use byteorder::ReadBytesExt;
use fragmented::{FragmentationUnitPacketType, FragmentedUnit};
use single_nalu::SingleNalUnit;
use utils::traits::{
    dynamic_sized_packet::DynamicSizedPacket,
    reader::{ReadFrom, ReadRemainingFrom},
    writer::WriteTo,
};

use crate::errors::RtpError;

#[derive(Debug, Clone, Copy)]
pub enum PayloadStructureType {
    SingleNALUPacket(u8),
    AggregationPacket(AggregationPacketType),
    FragmentationUnit(FragmentationUnitPacketType),
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
    type Error = RtpError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            v if (1..=23).contains(&v) => Ok(Self::SingleNALUPacket(v)),
            v if (24..=27).contains(&v) => Ok(Self::AggregationPacket(
                AggregationPacketType::try_from(v).unwrap(),
            )),
            v if v == 28 || v == 29 => Ok(Self::FragmentationUnit(
                FragmentationUnitPacketType::try_from(v).unwrap(),
            )),
            v => Err(RtpError::InvalidH264PacketType(v)),
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
            Self::Aggregated(nalu) => match nalu {
                AggregationNalUnits::StapA(packet) => packet.header,
                AggregationNalUnits::StapB(packet) => packet.header,
                AggregationNalUnits::Mtap16(packet) => packet.header,
                AggregationNalUnits::Mtap24(packet) => packet.header,
            },
            Self::Fragmented(nalu) => match nalu {
                FragmentedUnit::FuA(packet) => packet.indicator,
                FragmentedUnit::FuB(packet) => packet.indicator,
            },
        }
    }
}

impl<R: io::Read> ReadFrom<R> for RtpH264NalUnit {
    type Error = RtpError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
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
    type Error = RtpError;
    fn write_to(&self, writer: W) -> Result<(), Self::Error> {
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
