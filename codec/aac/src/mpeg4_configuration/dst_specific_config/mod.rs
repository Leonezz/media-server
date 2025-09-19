use utils::traits::fixed_packet::FixedBitwisePacket;

pub mod reader;
pub mod writer;

/// @see: 10.5.1 Decoder Configuration (DSTSpecificConfig())
#[derive(Debug, Clone)]
pub struct DSTSpecificConfig {
    pub dsddst_coded: bool, // 1 bit
    pub n_channels: u16,    // 14 bits
    pub reserved: bool,     // 1 bit
}

impl FixedBitwisePacket for DSTSpecificConfig {
    fn bits_count() -> usize {
        1 + // DSDDST_Coded
        14 + // N_Channels
        1 // reserved
    }
}
