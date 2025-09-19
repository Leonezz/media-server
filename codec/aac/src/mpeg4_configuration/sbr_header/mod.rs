use utils::traits::{
    dynamic_sized_packet::DynamicSizedBitsPacket, fixed_packet::FixedBitwisePacket,
};

pub mod reader;
pub mod writer;

/// @see: Table 4.63 – Syntax of sbr_header()
#[derive(Debug, Clone)]
pub struct SbrHeader {
    pub bs_amp_res: bool,  // 1 bit
    pub bs_start_freq: u8, // 4 bits
    pub bs_stop_freq: u8,  // 4 bits
    pub bs_xover_band: u8, // 3 bits
    pub bs_reserved: u8,   // 2 bits
    #[allow(unused)]
    bs_header_extra_1: bool, // 1 bit
    #[allow(unused)]
    bs_header_extra_2: bool, // 1 bit
    // if bs_header_extra_1 {
    pub extra1: Option<BsHeaderExtra1>,
    // }
    // if bs_header_extra_2 {
    pub extra2: Option<BsHeaderExtra2>,
    // }
}

impl DynamicSizedBitsPacket for SbrHeader {
    fn get_packet_bits_count(&self) -> usize {
        1 + // bs_amp_res
        4 + // bs_start_freq
        4 + // bs_stop_freq
        3 + // bs_xover_band
        2 + // bs_reserved
        1 + // bs_header_extra_1
        1 + // bs_header_extra_2
        self.extra1.map_or(0, |_| BsHeaderExtra1::bits_count()) +
        self.extra2.map_or(0, |_| BsHeaderExtra2::bits_count())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BsHeaderExtra1 {
    pub bs_freq_scale: u8,    // 2 bits
    pub bs_alter_scale: bool, // 1 bit
    pub bs_noise_bands: u8,   // 2 bits
}

impl FixedBitwisePacket for BsHeaderExtra1 {
    fn bits_count() -> usize {
        2 + // bs_freq_scale
        1 + // bs_alter_scale
        2 // bs_noise_bands
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BsHeaderExtra2 {
    pub bs_limiter_bands: u8,    // 2 bits
    pub bs_limiter_gains: u8,    // 2 bits
    pub bs_interpol_freq: bool,  // 1 bit
    pub bs_smoothing_mode: bool, // 1 bit
}

impl FixedBitwisePacket for BsHeaderExtra2 {
    fn bits_count() -> usize {
        2 + // bs_limiter_bands
        2 + // bs_limiter_gains
        1 + // bs_interpol_freq
        1 // bs_smoothing_mode
    }
}
