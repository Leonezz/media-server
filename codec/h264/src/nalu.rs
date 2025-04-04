use std::{fmt, io};

use byteorder::{ReadBytesExt, WriteBytesExt};
use tokio_util::bytes::Bytes;
use utils::traits::{
    dynamic_sized_packet::DynamicSizedPacket,
    fixed_packet::FixedPacket,
    reader::{ReadExactFrom, ReadFrom, ReadRemainingFrom},
    writer::WriteTo,
};

use crate::errors::H264CodecError;

// @see: Recommendation  ITU-T H.264 (V15) (08/2024)   – Coding of moving video
/// Table 7-1 – NAL unit type codes, syntax element categories, and NAL unit type classes
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NALUType {
    NonIDRSlice = 1,
    DataPartitionASlice = 2,
    DataPartitionBSlice = 3,
    DataPartitionCSlice = 4,
    IDRSlice = 5,
    SEI = 6,
    SPS = 7,
    PPS = 8,
    AccessUnitDelimiter = 9,
    EndOfSequence = 10,
    EndOfStream = 11,
    FillerData = 12,
    SPSExtension = 13,
    PrefixNALU = 14,
    SubsetSPS = 15,
    DPS = 16,
    SliceWithoutPartitioning = 19,
    SliceExtension = 20,
    SliceExtensionForDepthViewOr3DAVCTextureView = 21,
    Unspecified(u8),
    Reserved(u8),
}

impl From<NALUType> for u8 {
    fn from(value: NALUType) -> Self {
        match value {
            NALUType::NonIDRSlice => 1,
            NALUType::DataPartitionASlice => 2,
            NALUType::DataPartitionBSlice => 3,
            NALUType::DataPartitionCSlice => 4,
            NALUType::IDRSlice => 5,
            NALUType::SEI => 6,
            NALUType::SPS => 7,
            NALUType::PPS => 8,
            NALUType::AccessUnitDelimiter => 9,
            NALUType::EndOfSequence => 10,
            NALUType::EndOfStream => 11,
            NALUType::FillerData => 12,
            NALUType::SPSExtension => 13,
            NALUType::PrefixNALU => 14,
            NALUType::SubsetSPS => 15,
            NALUType::DPS => 16,
            NALUType::SliceWithoutPartitioning => 19,
            NALUType::SliceExtension => 20,
            NALUType::SliceExtensionForDepthViewOr3DAVCTextureView => 21,
            NALUType::Unspecified(v) | NALUType::Reserved(v) => v,
        }
    }
}

pub const H264_NALU_TYPE_U8_MASK: u8 = 0b11111;

impl TryFrom<u8> for NALUType {
    type Error = H264CodecError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value & H264_NALU_TYPE_U8_MASK {
            1 => Ok(Self::NonIDRSlice),
            2 => Ok(Self::DataPartitionASlice),
            3 => Ok(Self::DataPartitionBSlice),
            4 => Ok(Self::DataPartitionCSlice),
            5 => Ok(Self::IDRSlice),
            6 => Ok(Self::SEI),
            7 => Ok(Self::SPS),
            8 => Ok(Self::PPS),
            9 => Ok(Self::AccessUnitDelimiter),
            10 => Ok(Self::EndOfSequence),
            11 => Ok(Self::EndOfStream),
            12 => Ok(Self::FillerData),
            13 => Ok(Self::SPSExtension),
            14 => Ok(Self::PrefixNALU),
            15 => Ok(Self::SubsetSPS),
            16 => Ok(Self::DPS),
            19 => Ok(Self::SliceWithoutPartitioning),
            20 => Ok(Self::SliceExtension),
            21 => Ok(Self::SliceExtensionForDepthViewOr3DAVCTextureView),
            v if v == 0 || (24..=31).contains(&v) => Ok(Self::Unspecified(v)),
            v if (17..=18).contains(&v) || (22..=23).contains(&v) => Ok(Self::Reserved(v)),
            v => Err(H264CodecError::UnknownNaluType(v)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NaluHeader {
    // 1 bit
    pub forbidden_zero_bit: bool,
    // 2 bits
    pub nal_ref_idc: u8,
    // 5 bits
    pub nal_unit_type: NALUType,
}

impl From<NaluHeader> for u8 {
    fn from(value: NaluHeader) -> Self {
        let mut result: u8 = value.nal_unit_type.into();
        result |= value.nal_ref_idc << 5;
        result |= (value.forbidden_zero_bit as u8) << 7;
        result
    }
}

impl TryFrom<u8> for NaluHeader {
    type Error = H264CodecError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let forbidden_zero_bit = ((value >> 7) & 0b1) == 0b1;
        let nal_ref_idc = (value >> 5) & 0b11;
        let nal_unit_type: NALUType = (value & H264_NALU_TYPE_U8_MASK).try_into()?;
        Ok(Self {
            forbidden_zero_bit,
            nal_ref_idc,
            nal_unit_type,
        })
    }
}

impl FixedPacket for NaluHeader {
    fn bytes_count() -> usize {
        1
    }
}

#[derive(Clone)]
pub struct NalUnit {
    pub header: NaluHeader,
    // bytes in body does not include the header byte
    pub body: Bytes,
}

impl fmt::Debug for NalUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "nal_header: {:?}, payload length: {}",
            self.header,
            self.body.len()
        )
    }
}

/// read all the remaining bytes as body, the header was read ahead
impl<R: io::Read> ReadRemainingFrom<NaluHeader, R> for NalUnit {
    type Error = H264CodecError;
    fn read_remaining_from(header: NaluHeader, mut reader: R) -> Result<Self, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        Ok(Self {
            header,
            body: Bytes::from(bytes),
        })
    }
}

impl<W: io::Write> WriteTo<W> for NalUnit {
    type Error = H264CodecError;
    fn write_to(&self, mut writer: W) -> Result<(), Self::Error> {
        writer.write_u8(self.header.into())?;
        writer.write_all(&self.body)?;
        Ok(())
    }
}

/// read extract bytes as body, the header was read ahead
impl<R: io::Read> ReadRemainingFrom<(NaluHeader, usize), R> for NalUnit {
    type Error = H264CodecError;
    fn read_remaining_from(
        header: (NaluHeader, usize),
        mut reader: R,
    ) -> Result<Self, Self::Error> {
        let (header, body_size) = header;
        let mut bytes = vec![0; body_size];
        reader.read_exact(&mut bytes)?;
        Ok(Self {
            header,
            body: Bytes::from(bytes),
        })
    }
}

/// read all from reader, including the header
/// assumes all bytes from the reader consists the nalu
impl<R: io::Read> ReadFrom<R> for NalUnit {
    type Error = H264CodecError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let first_byte = reader.read_u8()?;
        let header: NaluHeader = first_byte.try_into()?;
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        Ok(Self {
            header,
            body: Bytes::from(bytes),
        })
    }
}

/// read extract bytes to consist a nalu
impl<R: io::Read> ReadExactFrom<R> for NalUnit {
    type Error = H264CodecError;
    fn read_exact_from(length: usize, mut reader: R) -> Result<Self, Self::Error> {
        let header: NaluHeader = reader.read_u8()?.try_into()?;
        Self::read_remaining_from((header, length - 1), reader)
    }
}

impl DynamicSizedPacket for NalUnit {
    fn get_packet_bytes_count(&self) -> usize {
        NaluHeader::bytes_count() + self.body.len()
    }
}
