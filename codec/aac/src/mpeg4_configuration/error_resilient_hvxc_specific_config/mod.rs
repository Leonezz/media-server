use utils::traits::dynamic_sized_packet::DynamicSizedBitsPacket;

use super::hvxc_specific_config::{HVXCrateMode, HVXCvarMode};
pub mod reader;
pub mod writer;
/// @see: Table 2.19 — Syntax of ErHvxcConfig()
#[derive(Debug, Clone, Copy)]
pub struct ErHvxcConfig {
    pub hvxc_var_mode: HVXCvarMode,   // 1 bit
    pub hvxc_rate_mode: HVXCrateMode, // 2 bits
    #[allow(unused)]
    extension_flag: bool, // 1 bit
    pub var_scalable_flag: Option<bool>, // 1 bit
}

impl DynamicSizedBitsPacket for ErHvxcConfig {
    fn get_packet_bits_count(&self) -> usize {
        1 + // HVXCvarMode
        2 + // HVXCrateMode
        1 + // extensionFlag
        self.var_scalable_flag.map_or(0, |_| 1)
    }
}

/// @see: 2.3.3Decoder configuration (ErrorResilientHvxcSpecificConfig)
#[derive(Debug, Clone)]
pub struct ErrorResilientHvxcSpecificConfig {
    #[allow(unused)]
    is_base_layer: bool, // 1 bit
    // if is_base_layer {
    pub config: Option<ErHvxcConfig>,
    // }
}

impl DynamicSizedBitsPacket for ErrorResilientHvxcSpecificConfig {
    fn get_packet_bits_count(&self) -> usize {
        1 + // is_base_layer
        self.config.map_or(0, |item| item.get_packet_bits_count())
    }
}
