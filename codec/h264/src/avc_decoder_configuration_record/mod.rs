use num::ToPrimitive;
use utils::traits::dynamic_sized_packet::DynamicSizedPacket;

use crate::{
    nalu::NalUnit,
    pps::Pps,
    sps::{Sps, chroma_format_idc::ChromaFormatIdc},
    sps_ext::SpsExt,
};

pub mod reader;
pub mod writer;

#[derive(Debug, Clone)]
pub struct ParameterSetInAvcDecoderConfigurationRecord<T> {
    pub sequence_parameter_set_length: u16, // u(16)
    pub nalu: NalUnit,
    pub parameter_set: T,
}

#[derive(Debug, Clone)]
pub struct SpsExtRelated {
    #[allow(unused)]
    reserved_6_bits_1: u8, // u(6), equal to 1
    pub chroma_format_idc: ChromaFormatIdc, // u(2)
    #[allow(unused)]
    reserved_5_bits_1: u8, // u(5), equal to 1
    pub bit_depth_luma_minus8: u8,          // u(3)
    _reserved_5_bits_1: u8,                 // u(5), equal to 1
    pub bit_depth_chroma_minus8: u8,        // u(3)
    #[allow(unused)]
    num_of_sequence_parameter_ext: u8, // u(8)
    pub sequence_parameter_set_ext: Vec<ParameterSetInAvcDecoderConfigurationRecord<SpsExt>>,
}

impl DynamicSizedPacket for SpsExtRelated {
    fn get_packet_bytes_count(&self) -> usize {
        1 + // reserved_6_bits_1 + chroma_format_idc
        1 + // reserved_5_bits_1 + bit_depth_luma_minus8
        1 + // _reserved_5_bits_1 + bit_depth_chroma_minus8
        1 + // num_of_sequence_parameter_ext
        self.sequence_parameter_set_ext
            .iter()
            .fold(0, |prev, item| prev + 2 + item.sequence_parameter_set_length.to_usize().unwrap())
    }
}

/// @see Information technology - Coding of audio-visual objects Part 15:  Advanced Video Coding (AVC) file format
/// Section 5.2.4.1.1 Syntax
#[derive(Debug, Clone)]
pub struct AvcDecoderConfigurationRecord {
    pub configuration_version: u8,  // u(8), equal to 1
    pub avc_profile_indication: u8, // u(8)
    pub profile_compatibility: u8,  // u(8)
    pub avc_level_indication: u8,   // u(8)
    #[allow(unused)]
    reserved_6_bits_1: u8, // u(6), equal to 1
    pub length_size_minus_one: u8,  // u(2), in {0, 1, 3}
    #[allow(unused)]
    reserved_3_bits_1: u8, // u(3), equal to 1
    #[allow(unused)]
    num_of_sequence_parameter_sets: u8, // u(5)
    pub sequence_parameter_sets: Vec<ParameterSetInAvcDecoderConfigurationRecord<Sps>>,
    #[allow(unused)]
    num_of_picture_parameter_sets: u8, // u(8)
    pub picture_parameter_sets: Vec<ParameterSetInAvcDecoderConfigurationRecord<Pps>>,
    pub sps_ext_related: Option<SpsExtRelated>,
}

impl DynamicSizedPacket for AvcDecoderConfigurationRecord {
    fn get_packet_bytes_count(&self) -> usize {
        1 + // configuration_version
        1 + // avc_profile_indication
        1 + // profile_compatibility
        1 + // avc_level_indication
        1 + // reserved_6_bits_1 + length_size_minus_one
        1 + // reserved_3_bits_1 + num_of_sequence_parameter_sets
        self.sequence_parameter_sets
            .iter()
            .fold(0, 
                |prev, item| prev + 2 + item.sequence_parameter_set_length.to_usize().unwrap()) +
        1 + // num_of_picture_parameter_sets
        self.picture_parameter_sets
            .iter()
            .fold(0,
                 |prev, item| prev + 2 + item.sequence_parameter_set_length.to_usize().unwrap()) +
        self.sps_ext_related.as_ref().map_or(0, |v| v.get_packet_bytes_count())
    }
}
