use utils::traits::{dynamic_sized_packet::DynamicSizedBitsPacket, fixed_packet::FixedBitwisePacket};

use crate::errors::AACCodecError;

use super::audio_specific_config::sampling_frequency_index::SamplingFrequencyIndex;
pub mod reader;
pub mod writer;
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum ObjectType {
    AACMain = 0,
    AACLC = 1,
    AACSSR = 2,
    AACLTP = 3,
}

impl From<ObjectType> for u8 {
    fn from(value: ObjectType) -> Self {
        match value {
            ObjectType::AACMain => 0,
            ObjectType::AACLC => 1,
            ObjectType::AACSSR => 2,
            ObjectType::AACLTP => 3
        }
    }
}

impl TryFrom<u8> for ObjectType {
    type Error = AACCodecError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::AACMain),
            1 => Ok(Self::AACLC),
            2 => Ok(Self::AACSSR),
            3 => Ok(Self::AACLTP),
            _ => Err(AACCodecError::UnknownObjectType(value))
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ChannelElement {
    pub is_cpe: bool,   // 1 bit
    pub tag_select: u8, // 4 bits
}

impl FixedBitwisePacket for ChannelElement {
    fn bits_count() -> usize {
        1 + 4
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ValidCCElement {
    pub is_ind_sw: bool, // 1 bit
    pub tag_select: u8,  // 4 bits
}

impl FixedBitwisePacket for ValidCCElement {
    fn bits_count() -> usize {
        1 + 4
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MatrixMixdownIdx {
    pub matrix_mixdown_idx: u8,       // 2 bits
    pub pseudo_surround_enable: bool, // 1 bit
}

impl FixedBitwisePacket for MatrixMixdownIdx {
    fn bits_count() -> usize {
        2 + 1
    }
}

/// @see: 4.4.1.1 Program config element
/// Table 4.2 – Syntax of program_config_element()
#[derive(Debug, Clone)]
pub struct ProgramConfigElement {
    pub element_instance_tag: u8,                         // 4 bits
    pub object_type: ObjectType,                          // 2 bits
    pub sampling_frequency_index: SamplingFrequencyIndex, // 4 bits
    #[allow(unused)]
    num_front_channel_elements: u8,                       // 4 bits
    #[allow(unused)]
    num_side_channel_elements: u8,                        // 4 bits
    #[allow(unused)]
    num_back_channel_elements: u8,                        // 4 bits
    #[allow(unused)]
    num_lfe_channel_elements: u8,                         // 2 bits
    #[allow(unused)]
    num_assoc_data_elements: u8,                          // 3 bits
    #[allow(unused)]
    num_valid_cc_elements: u8,                            // 4 bits
    #[allow(unused)]
    mono_mixdown_present: bool,                           // 1 bit
    /// if mono_mixdown_present {
    pub mono_mixdown_element_number: Option<u8>, // 4 bits
    /// }
    #[allow(unused)]
    stereo_mixdown_present: bool,                // 1 bit
    /// if stereo_mixdown_present {
    pub stereo_mixdown_element_number: Option<u8>, // 4 bits
    /// }
    #[allow(unused)]
    matrix_mixdown_idx_present: bool,            // 1 bit
    /// if matrix_mixdown_idx_present {
    pub matrix_mix_down_idx: Option<MatrixMixdownIdx>,
    /// }
    pub front_channel_elements: Vec<ChannelElement>,
    pub side_channel_elements: Vec<ChannelElement>,
    pub back_channel_elements: Vec<ChannelElement>,
    pub lfe_element_tag_select: Vec<u8>,        // 4 bits
    pub assoc_data_element_tag_select: Vec<u8>, // 4 bits
    pub valid_cc_elements: Vec<ValidCCElement>,
    // byte_alignment
    #[allow(unused)]
    comment_field_bytes: u8,         // 8 bits
    pub comment_field_data: Vec<u8>, // 8 bits
}

impl DynamicSizedBitsPacket for ProgramConfigElement {
    fn get_packet_bits_count(&self) -> usize {
        4 + // element_instance_tag
        2 + // object_type
        4 + // sampling_frequency_index
        4 + // num_front_channel_elements
        4 + // num_side_channel_elements
        4 + // num_back_channel_elements
        2 + // num_lfe_channel_elements
        3 + // num_assoc_data_elements
        4 + // num_valid_cc_elements
        1 + // mono_mixdown_present
        self.mono_mixdown_element_number.map_or(0, |_| 4) +
        1 + // stereo_mixdown_present
        self.stereo_mixdown_element_number.map_or(0, |_| 4) +
        1 + // matrix_mixdown_idx_present
        self.matrix_mix_down_idx.as_ref().map_or(0, |_| MatrixMixdownIdx::bits_count()) + 
        self.front_channel_elements.len() * ChannelElement::bits_count() + 
        self.side_channel_elements.len() * ChannelElement::bits_count() +
        self.back_channel_elements.len() * ChannelElement::bits_count() +
        self.lfe_element_tag_select.len() * 4 +
        self.assoc_data_element_tag_select.len() * 4 +
        self.valid_cc_elements.len() * ValidCCElement::bits_count() +
        // TODO: what about the byte alignment?
        8 + // comment_field_bytes
        self.comment_field_data.len() * 8
    }
}