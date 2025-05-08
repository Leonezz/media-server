use utils::traits::fixed_packet::FixedPacket;

use crate::{
    errors::H264CodecError,
    nalu_type::{H264_NALU_TYPE_U8_MASK, NALUType},
};

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
