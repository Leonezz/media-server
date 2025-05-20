use utils::traits::dynamic_sized_packet::DynamicSizedBitsPacket;

use super::program_config_element::ProgramConfigElement;
pub mod writer;
pub mod reader;
#[derive(Debug, Clone)]
pub struct GAExtension {
    pub num_of_sub_frame: Option<u8>,                       // 5 bits
    pub layer_length: Option<u16>,                          // 11 bits
    pub aac_section_data_resilience_flag: Option<bool>,     // 1 bit
    pub aac_scalefactor_data_resilience_flag: Option<bool>, // 1 bit
    pub aac_spectral_data_resilience_flag: Option<bool>,    // 1 bit
    pub extension_flag3: bool,
    // if extension_flag3 {
    // tbd
    //}
}

impl DynamicSizedBitsPacket for GAExtension {
    fn get_packet_bits_count(&self) -> usize {
        self.num_of_sub_frame.map_or(0, |_| 5)
            + self.layer_length.map_or(0, |_| 11)
            + self.aac_section_data_resilience_flag.map_or(0, |_| 1)
            + self.aac_scalefactor_data_resilience_flag.map_or(0, |_| 1)
            + self.aac_spectral_data_resilience_flag.map_or(0, |_| 1)
            + 1 // extensionFlag3
    }
}

#[derive(Debug, Clone)]
pub struct GASpecificConfig {
    pub frame_length_flag: bool,     // 1 bit
    pub depends_on_core_coder: bool, // 1 bit
    /// if depends_on_core_coder {
    pub core_coder_delay: Option<u16>, // 14 bits
    /// }
    #[allow(unused)]
    extension_flag: bool,   // 1 bit
    pub program_config_element: Option<ProgramConfigElement>,
    pub layer_nr: Option<u8>, // 3 bits
    /// if extension_flag {
    pub extension: Option<GAExtension>,
    // }
}

impl DynamicSizedBitsPacket for GASpecificConfig {
    fn get_packet_bits_count(&self) -> usize {
        1 + // frameLengthFlag
        1 + // dependsOnCoreCoder
        self.core_coder_delay.map_or(0, |_| 14) +
        1 + // extensionFlag
        self.program_config_element.as_ref().map_or(0, |item| item.get_packet_bits_count()) +
        self.layer_nr.map_or(0, |_| 3) + 
        self.extension.as_ref().map_or(0, |item| item.get_packet_bits_count())
    }
}