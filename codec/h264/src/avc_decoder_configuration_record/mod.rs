use crate::{
    nalu::NalUnit,
    pps::Pps,
    sps::{Sps, chroma_format_idc::ChromaFormatIdc},
    sps_ext::SpsExt,
};

pub mod reader;
pub mod writer;

#[derive(Debug)]
pub struct ParameterSetInAvcDecoderConfigurationRecord<T> {
    pub sequence_parameter_set_length: u16, // u(16)
    pub nalu: NalUnit,
    pub parameter_set: T,
}

#[derive(Debug)]
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

/// @see Information technology - Coding of audio-visual objects Part 15:  Advanced Video Coding (AVC) file format
/// Section 5.2.4.1.1 Syntax
#[derive(Debug)]
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
