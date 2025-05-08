use crate::errors::H264CodecError;

/// @see: Recommendation  ITU-T H.264 (V15) (08/2024)   – Coding of moving video
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
