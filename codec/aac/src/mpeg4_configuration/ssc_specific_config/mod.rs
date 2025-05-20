use utils::traits::dynamic_sized_packet::DynamicSizedBitsPacket;

pub mod reader;
pub mod writer;
/// @see: Table 8.1 — Syntax of SSCSpecificConfig()
#[derive(Debug, Clone)]
pub struct SSCSpecificConfig {
    pub decoder_level: u8,    // 2 bits
    pub update_rate: u8,      // 4 bits
    pub synthesis_method: u8, // 2 bits
    pub mode_ext: Option<u8>, // 2 bits
    pub reserved: Option<u8>, // 2 bits
}

impl DynamicSizedBitsPacket for SSCSpecificConfig {
    fn get_packet_bits_count(&self) -> usize {
        2 + // decoder_level
        4 + // update_rate
        2 + // synthesis_method
        self.mode_ext.map_or(0, |_| 2) +
        self.reserved.map_or(0, |_| 2)
    }
}
