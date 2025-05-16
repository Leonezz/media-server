use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io;
use tokio_util::bytes::Bytes;
use utils::traits::{
    dynamic_sized_packet::DynamicSizedPacket, fixed_packet::FixedPacket, reader::ReadRemainingFrom,
    writer::WriteTo,
};

use super::errors::RtpH264Error;

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum FragmentationUnitPacketType {
    FUA = 28,
    FUB = 29,
}

impl From<FragmentationUnitPacketType> for u8 {
    fn from(value: FragmentationUnitPacketType) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for FragmentationUnitPacketType {
    type Error = RtpH264Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            28 => Ok(Self::FUA),
            29 => Ok(Self::FUB),
            v => Err(RtpH264Error::InvalidH264PacketType(v)),
        }
    }
}

// @see: RFC 6184 5.8. Fragmentation Units (FUs)
/// +---------------+
/// |0|1|2|3|4|5|6|7|
/// +-+-+-+-+-+-+-+-+
/// |S|E|R|   Type  |
/// +---------------+
#[derive(Debug, Clone, Copy)]
pub struct FUHeader {
    pub start_bit: bool,
    pub end_bit: bool,
    pub reserved_bit: bool,
    pub nalu_type: u8,
}

impl From<FUHeader> for u8 {
    fn from(value: FUHeader) -> Self {
        ((value.start_bit as u8) << 7)
            | ((value.end_bit as u8) << 6)
            | ((value.reserved_bit as u8) << 5)
            | (value.nalu_type & 0b1_1111)
    }
}

impl From<u8> for FUHeader {
    fn from(value: u8) -> Self {
        Self {
            start_bit: ((value >> 7) & 0b1) == 0b1,
            end_bit: ((value >> 6) & 0b1) == 0b1,
            reserved_bit: ((value >> 5) & 0b1) == 0b1,
            nalu_type: value & 0b1_1111,
        }
    }
}

impl FixedPacket for FUHeader {
    fn bytes_count() -> usize {
        1
    }
}

// FU-A
///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// | FU indicator  |   FU header   |                               |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+                               |
/// |                                                               |
/// |                          FU payload                           |
/// |                                                               |
/// |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                               :...OPTIONAL RTP padding        |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Debug)]
pub struct FUAPacket {
    pub indicator: u8,
    pub fu_header: FUHeader,
    pub payload: Bytes,
}

impl<R: io::Read> ReadRemainingFrom<u8, R> for FUAPacket {
    type Error = RtpH264Error;
    fn read_remaining_from(indicator: u8, reader: &mut R) -> Result<Self, Self::Error> {
        let fu_header: FUHeader = reader.read_u8()?.into();
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        Ok(Self {
            indicator,
            fu_header,
            payload: Bytes::from(bytes),
        })
    }
}

impl<W: io::Write> WriteTo<W> for FUAPacket {
    type Error = RtpH264Error;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u8(self.indicator)?;
        writer.write_u8(self.fu_header.into())?;
        writer.write_all(&self.payload)?;
        Ok(())
    }
}

impl DynamicSizedPacket for FUAPacket {
    fn get_packet_bytes_count(&self) -> usize {
        1 // FU indicator
        + FUHeader::bytes_count()
        + self.payload.len()
    }
}

// FU-B
///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// | FU indicator  |   FU header   |              DON              |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                                                               |
/// |                         FU payload                            |
/// |                                                               |
/// |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                               :...OPTIONAL RTP padding        |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Debug)]
pub struct FUBPacket {
    pub indicator: u8,
    pub fu_header: FUHeader,
    pub decode_order_number: u16,
    pub payload: Bytes,
}

impl<R: io::Read> ReadRemainingFrom<u8, R> for FUBPacket {
    type Error = RtpH264Error;
    fn read_remaining_from(indicator: u8, reader: &mut R) -> Result<Self, Self::Error> {
        let fu_header: FUHeader = reader.read_u8()?.into();
        let decode_order_number = reader.read_u16::<BigEndian>()?;
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        Ok(Self {
            indicator,
            fu_header,
            decode_order_number,
            payload: Bytes::from(bytes),
        })
    }
}

impl<W: io::Write> WriteTo<W> for FUBPacket {
    type Error = RtpH264Error;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u8(self.indicator)?;
        writer.write_u8(self.fu_header.into())?;
        writer.write_u16::<BigEndian>(self.decode_order_number)?;
        writer.write_all(&self.payload)?;
        Ok(())
    }
}

impl DynamicSizedPacket for FUBPacket {
    fn get_packet_bytes_count(&self) -> usize {
        1 // FU indicator
        + FUHeader::bytes_count()
        + 2 // don
        + self.payload.len()
    }
}

#[derive(Debug)]
pub enum FragmentedUnit {
    FuA(FUAPacket),
    FuB(FUBPacket),
}

impl<R: io::Read> ReadRemainingFrom<FragmentationUnitPacketType, R> for FragmentedUnit {
    type Error = RtpH264Error;
    fn read_remaining_from(
        header: FragmentationUnitPacketType,
        reader: &mut R,
    ) -> Result<Self, Self::Error> {
        match header {
            FragmentationUnitPacketType::FUA => Ok(Self::FuA(FUAPacket::read_remaining_from(
                header.into(),
                reader,
            )?)),
            FragmentationUnitPacketType::FUB => Ok(Self::FuB(FUBPacket::read_remaining_from(
                header.into(),
                reader,
            )?)),
        }
    }
}

impl<W: io::Write> WriteTo<W> for FragmentedUnit {
    type Error = RtpH264Error;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        match self {
            Self::FuA(packet) => packet.write_to(writer),
            Self::FuB(packet) => packet.write_to(writer),
        }
    }
}

impl DynamicSizedPacket for FragmentedUnit {
    fn get_packet_bytes_count(&self) -> usize {
        match self {
            Self::FuA(packet) => packet.get_packet_bytes_count(),
            Self::FuB(packet) => packet.get_packet_bytes_count(),
        }
    }
}
