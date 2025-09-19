use std::ops::Rem;

use num::ToPrimitive;
use utils::traits::dynamic_sized_packet::DynamicSizedBitsPacket;

pub mod reader;
pub mod writer;
#[derive(Debug, Clone)]
pub struct AuxData {
    pub aux_size: u32,     // 32 bits
    pub aux_data: Vec<u8>, // 8 bits
}

impl DynamicSizedBitsPacket for AuxData {
    fn get_packet_bits_count(&self) -> usize {
        32 + // aux_size
        self.aux_data.len() * 8
    }
}

/// @see: Table 11.1 — Syntax of ALSSpecificConfig and Table 11.9 — Elements of ALSSpecificConfig
#[derive(Debug, Clone)]
pub struct ALSSpecificConfig {
    pub als_id: u32,                // 32 bits
    pub samp_freq: u32,             // 32 bits
    pub samples: u32,               // 32 bits
    pub channels: u16,              // 16 bits
    pub file_type: u8,              // 3 bits
    pub resolution: u8,             // 3 bits
    pub floating: bool,             // 1 bit
    pub msb_first: bool,            // 1 bit
    pub frame_length: u16,          // 16 bits
    pub random_access: u8,          // 8 bits
    pub ra_flag: u8,                // 2 bits
    pub adapt_order: bool,          // 1 bit
    pub coef_table: u8,             // 2 bits
    pub long_term_prediction: bool, // 1 bit
    pub max_order: u16,             // 10 bits
    pub block_switching: u8,        // 2 bits
    pub bgmc_mode: bool,            // 1 bit
    pub sb_part: bool,              // 1 bit
    pub joint_stereo: bool,         // 1 bit
    pub mc_coding: bool,            // 1 bit
    pub chan_config: bool,          // 1 bit
    pub chan_sort: bool,            // 1 bit
    pub crc_enabled: bool,          // 1 bit
    pub rlslms: bool,               // 1 bit
    pub reserved: u8,               // 5 bits
    pub aux_data_enabled: bool,     // 1 bit
    // if chan_config {
    pub chan_config_info: Option<u16>, // 16 bits
    // }
    pub chan_pos: Option<Vec<u16>>, // ceil[log2(channels+1)] = 1..16 bits
    #[allow(unused)]
    byte_align: u8,                 // 0..7 bits, relative to the start of ALSSpecificConfig
    pub header_size: u32,           // 32 bits
    pub trailer_size: u32,          // 32 bits
    pub orig_header: Vec<u8>,       // 8 bits
    pub orig_trailer: Vec<u8>,      // 8 bits
    // if crc_enabled {
    pub crc: Option<u32>, // 32 bits
    // }
    pub ra_unit_size: Option<Vec<u32>>, // 32 bits
    // if aux_data_enabled {
    pub aux_data: Option<AuxData>,
    // }
}

impl DynamicSizedBitsPacket for ALSSpecificConfig {
    fn get_packet_bits_count(&self) -> usize {
        let result = 32 + // als_id
            32 + // samp_freq
            32 + // samples
            16 + // channels
            3 + // file_type
            3 + // resolution
            1 + // floating
            1 + // msb_first
            16 + // frame_length
            8 + // random_access
            2 + // ra_flag
            1 + // adapt_order
            2 + // coef_table
            1 + // long_term_prediction
            10 + // max_order
            2 + // block_switching
            1 + // bgmc_mode
            1 + // sb_part
            1 + // joint_stereo
            1 + // mc_coding
            1 + // chan_config
            1 + // chan_sort
            1 + // crc_enabled
            1 + // RLSLMS
            5 + // reserved
            1 + // aux_data_enabled
            self.chan_config_info.map_or(0, |_| 16);
        let chan_pos_bits = self
            .channels
            .checked_add(1)
            .and_then(|v| v.to_f64())
            .and_then(|v| v.log2().ceil().to_usize())
            .unwrap();
        result + 
            self
                .chan_pos
                .as_ref()
                .map_or(0, |item| item.len() * chan_pos_bits) +
            8_usize.checked_sub(result.rem(8)).unwrap() + // byte_align
            32 + // header_size
            32 + // trailer_size
            self.orig_header.len() * 8 +
            self.orig_trailer.len() * 8 +
            self.crc.map_or(0, |_| 32) +
            self.ra_unit_size.as_ref().map_or(0, |item| item.len() * 32) +
            self.aux_data.as_ref().map_or(0, |item| item.get_packet_bits_count())
    }
}
