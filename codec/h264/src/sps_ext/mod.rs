use bitstream_io::BitWrite;
use num::ToPrimitive;
use tokio_util::bytes::Bytes;
use utils::traits::{dynamic_sized_packet::DynamicSizedBitsPacket, writer::BitwiseWriteTo};

use crate::{
    exp_golomb::find_ue_bits_count, nalu::NalUnit, nalu_header::NaluHeader, rbsp::rbsp_to_sodb,
};

pub mod reader;
pub mod writer;

#[derive(Debug, Clone)]
pub struct AuxFormatIdcRelated {
    pub bit_depth_aux_minus8: u8,     // ue(v), in [0, 4]
    pub alpha_incr_flag: bool,        // u(1)
    pub alpha_opaque_value: u16,      // u(v), v = bit_depth_aux_minus8 + 9
    pub alpha_transparent_value: u16, // u(v), v = bit_depth_aux_minus8 + 9
}

impl DynamicSizedBitsPacket for AuxFormatIdcRelated {
    fn get_packet_bits_count(&self) -> usize {
        find_ue_bits_count(self.bit_depth_aux_minus8).unwrap() +
        1 + // alpha_incr_flag
        self.bit_depth_aux_minus8.to_usize().unwrap() + 9 +
        self.bit_depth_aux_minus8.to_usize().unwrap() + 9
    }
}

/// @see: Recommendation  ITU-T H.264 (V15) (08/2024)   – Coding of moving video
/// Section 7.3.2.1.2 Sequence parameter set extension RBSP syntax
#[derive(Debug, Clone)]
pub struct SpsExt {
    pub seq_parameter_set_id: u64, // ue(v)
    pub aux_format_idc: u8,        // ue(v), in [0, 3]
    /// if( aux_format_idc != 0 ) {
    pub aux_format_idc_related: Option<AuxFormatIdcRelated>,
    /// }
    pub additional_extension_flag: bool, // u(1)
}

impl DynamicSizedBitsPacket for SpsExt {
    fn get_packet_bits_count(&self) -> usize {
        find_ue_bits_count(self.seq_parameter_set_id).unwrap()
            + find_ue_bits_count(self.aux_format_idc).unwrap()
            + self
                .aux_format_idc_related
                .as_ref()
                .map_or(0, |v| v.get_packet_bits_count())
            + 1 // additional_extension_flag
    }
}

impl From<&SpsExt> for NalUnit {
    fn from(value: &SpsExt) -> Self {
        let mut bytes = Vec::with_capacity(
            value
                .get_packet_bits_count()
                .checked_add(8)
                .and_then(|v| v.checked_div(8))
                .unwrap(),
        );
        let mut writer = bitstream_io::BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
        value.write_to(writer.by_ref()).unwrap();
        writer.write_bit(true).unwrap();
        writer.byte_align().unwrap();
        let bytes = rbsp_to_sodb(&bytes);
        Self {
            header: NaluHeader {
                forbidden_zero_bit: false,
                nal_ref_idc: 3,
                nal_unit_type: crate::nalu_type::NALUType::SPSExtension,
            },
            body: Bytes::from_owner(bytes),
        }
    }
}
