use utils::traits::dynamic_sized_packet::DynamicSizedBitsPacket;

use super::program_config_element::ProgramConfigElement;
pub mod reader;
pub mod writer;
/// @see: Table 12.1 — Syntax of SLSSpecificConfig
#[derive(Debug, Clone)]
pub struct SLSSpecificConfig {
    pub pcm_word_length: u8,    // 3 bits
    pub aac_core_present: bool, // 1 bit
    pub lle_main_stream: bool,  // 1 bit
    pub reserved_bit: bool,     // 1 bit
    pub frame_length: u8,       // 3 bits
    // if !channelConfiguration {
    pub program_config_element: Option<ProgramConfigElement>,
    // }
}

impl DynamicSizedBitsPacket for SLSSpecificConfig {
    fn get_packet_bits_count(&self) -> usize {
        3 + // pcmWordLength
        1 + // aac_core_present
        1 + // lle_main_stream
        1 + // reserved_bit
        3 + // frameLength
        self.program_config_element.as_ref().map_or(0, |item| item.get_packet_bits_count())
    }
}
