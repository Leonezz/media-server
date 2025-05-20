use utils::traits::{dynamic_sized_packet::DynamicSizedBitsPacket, fixed_packet::FixedBitwisePacket};

use super::celp_header::{CelpBWSenhHeader, CelpHeader};


pub mod reader;
pub mod writer;
/// @see: Table 3.10 — Syntax of CelpSpecificConfig ()
#[derive(Debug, Clone)]
pub struct CelpSpecificConfig {
    #[allow(unused)]
    is_base_layer: bool, // 1 bit
    // if is_base_layer {
    pub celp_header: Option<CelpHeader>,
    // } else {
    is_bws_layer: Option<bool>,
    //   if is_bws_layer {
    pub celp_bwsenh_header: Option<CelpBWSenhHeader>,
    //   } else {
    pub celp_brs_id: Option<u8>, // 2 bits
                                 //   }
                                 //}
}


impl DynamicSizedBitsPacket for CelpSpecificConfig {
    fn get_packet_bits_count(&self) -> usize {
        1 + // isBaseLayer
        self.celp_header.as_ref().map_or(0, |item| item.get_packet_bits_count()) +
        self.is_bws_layer.map_or(0, |_| 1) + 
        self.celp_bwsenh_header.map_or(0, |_| CelpBWSenhHeader::bits_count()) +
        self.celp_brs_id.map_or(0, |_| 2)
    }
}
