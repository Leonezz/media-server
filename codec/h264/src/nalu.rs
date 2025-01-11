use std::io;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use tokio_util::bytes::Bytes;
use utils::traits::{
    reader::{ReadExactFrom, ReadFrom, ReadRemainingFrom},
    writer::WriteTo,
};

use crate::errors::{H264CodecError, H264CodecResult};

///! @see: Recommendation  ITU-T H.264 (V15) (08/2024)   – Coding of moving video
/// Table 7-1 – NAL unit type codes, syntax element categories, and NAL unit type classes
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
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

impl Into<u8> for NALUType {
    fn into(self) -> u8 {
        match self {
            Self::NonIDRSlice => 1,
            Self::DataPartitionASlice => 2,
            Self::DataPartitionBSlice => 3,
            Self::DataPartitionCSlice => 4,
            Self::IDRSlice => 5,
            Self::SEI => 6,
            Self::SPS => 7,
            Self::PPS => 8,
            Self::AccessUnitDelimiter => 9,
            Self::EndOfSequence => 10,
            Self::EndOfStream => 11,
            Self::FillerData => 12,
            Self::SPSExtension => 13,
            Self::PrefixNALU => 14,
            Self::SubsetSPS => 15,
            Self::DPS => 16,
            Self::SliceWithoutPartitioning => 19,
            Self::SliceExtension => 20,
            Self::SliceExtensionForDepthViewOr3DAVCTextureView => 21,
            Self::Unspecified(v) | Self::Reserved(v) => v,
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
            v if v == 0 || (v >= 24 && v <= 31) => Ok(Self::Unspecified(v)),
            v if (v >= 17 && v <= 18) || (v >= 22 && v <= 23) => Ok(Self::Reserved(v)),
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

impl Into<u8> for NaluHeader {
    fn into(self) -> u8 {
        let mut result: u8 = self.nal_unit_type.into();
        result |= self.nal_ref_idc << 5;
        result |= (self.forbidden_zero_bit as u8) << 7;
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

#[derive(Debug)]
pub struct NalUnit {
    pub header: NaluHeader,
    // bytes in body does not include the header byte
    pub body: Bytes,
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
        let mut bytes = Vec::with_capacity(body_size);
        bytes.resize(body_size, 0);
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
