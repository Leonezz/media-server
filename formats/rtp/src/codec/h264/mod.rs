pub mod aggregation;
pub mod fragmented;
pub mod packet;
pub mod single_nalu;
pub(crate) mod util;
use std::io;

use aggregation::{AggregationNalUnits, AggregationPacketType};
use byteorder::{ReadBytesExt, WriteBytesExt};
use fragmented::{FragmentationUnitPacketType, FragmentedUnit};
use single_nalu::SingleNalUnit;
use utils::traits::{
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

impl Into<u8> for PayloadStructureType {
    fn into(self) -> u8 {
        match self {
            Self::SingleNALUPacket(v) => v,
            Self::AggregationPacket(v) => v.into(),
            Self::FragmentationUnit(v) => v.into(),
        }
    }
}

impl TryFrom<u8> for PayloadStructureType {
    type Error = RtpError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            v if v >= 1 && v <= 23 => Ok(Self::SingleNALUPacket(v)),
            v if v >= 24 && v <= 27 => Ok(Self::AggregationPacket(
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
pub enum H264RtpNalUnit {
    SingleNalu(SingleNalUnit),
    Aggregated(AggregationNalUnits),
    Fragmented(FragmentedUnit),
}

#[derive(Debug)]
pub struct H264RtpPayload {
    pub payload_structure: PayloadStructureType,
    pub packet: H264RtpNalUnit,
}

impl<R: io::Read> ReadFrom<R> for H264RtpPayload {
    type Error = RtpError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let first_byte = reader.read_u8()?;
        let payload_structure: PayloadStructureType = first_byte.try_into()?;
        let packet: H264RtpNalUnit = match payload_structure {
            PayloadStructureType::SingleNALUPacket(header) => H264RtpNalUnit::SingleNalu(
                SingleNalUnit::read_remaining_from(header.into(), reader)?,
            ),
            PayloadStructureType::AggregationPacket(header) => H264RtpNalUnit::Aggregated(
                AggregationNalUnits::read_remaining_from(header.into(), reader)?,
            ),
            PayloadStructureType::FragmentationUnit(header) => H264RtpNalUnit::Fragmented(
                FragmentedUnit::read_remaining_from(header.into(), reader)?,
            ),
        };

        Ok(Self {
            payload_structure,
            packet,
        })
    }
}

impl<W: io::Write> WriteTo<W> for H264RtpPayload {
    type Error = RtpError;
    fn write_to(&self, mut writer: W) -> Result<(), Self::Error> {
        writer.write_u8(self.payload_structure.into())?;
        match &self.packet {
            H264RtpNalUnit::SingleNalu(packet) => packet.write_to(writer),
            H264RtpNalUnit::Aggregated(packet) => packet.write_to(writer),
            H264RtpNalUnit::Fragmented(packet) => packet.write_to(writer),
        }
    }
}
