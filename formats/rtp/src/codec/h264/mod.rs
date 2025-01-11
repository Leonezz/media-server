pub mod aggregation;
pub mod fragmented;
pub mod single_nalu;
pub(crate) mod util;
use std::io;

use aggregation::{AggregationPacket, AggregationPacketType};
use byteorder::{ReadBytesExt, WriteBytesExt};
use fragmented::{FUPacket, FragmentationUnitPacketType};
use single_nalu::SingleNaluPacket;
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
pub enum H264RtpPacket {
    SingleNalu(SingleNaluPacket),
    Aggregated(AggregationPacket),
    Fragmented(FUPacket),
}

#[derive(Debug)]
pub struct H264RtpPayload {
    pub payload_structure: PayloadStructureType,
    pub packet: H264RtpPacket,
}

impl<R: io::Read> ReadFrom<R> for H264RtpPayload {
    type Error = RtpError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let first_byte = reader.read_u8()?;
        let payload_structure: PayloadStructureType = first_byte.try_into()?;
        let packet: H264RtpPacket = match payload_structure {
            PayloadStructureType::SingleNALUPacket(header) => H264RtpPacket::SingleNalu(
                SingleNaluPacket::read_remaining_from(header.into(), reader)?,
            ),
            PayloadStructureType::AggregationPacket(header) => H264RtpPacket::Aggregated(
                AggregationPacket::read_remaining_from(header.into(), reader)?,
            ),
            PayloadStructureType::FragmentationUnit(header) => {
                H264RtpPacket::Fragmented(FUPacket::read_remaining_from(header.into(), reader)?)
            }
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
            H264RtpPacket::SingleNalu(packet) => packet.write_to(writer),
            H264RtpPacket::Aggregated(packet) => packet.write_to(writer),
            H264RtpPacket::Fragmented(packet) => packet.write_to(writer),
        }
    }
}
