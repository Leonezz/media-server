
use num::ToPrimitive;
use utils::traits::dynamic_sized_packet::DynamicSizedBitsPacket;

use super::sbr_header::SbrHeader;

pub mod reader;
pub mod writer;

pub(crate) const ELDEXT_TERM: u8 = 0;

/// @see: Table 4.180 – Syntax of ELDSpecificConfig ()
#[derive(Debug, Clone)]
pub struct ELDSpecificConfig {
    pub frame_length_flag: bool,                    // 1 bit
    pub aac_section_data_resilience_flag: bool,     // 1 bit
    pub aac_scalefactor_data_resilience_flag: bool, // 1 bit
    pub aac_spectral_data_resilience_flag: bool,    // 1 bit
    #[allow(unused)]
    ld_sbr_present_flag: bool,                      // 1 bit
    pub ld_sbr: Option<LdSbr>,
    pub eld_ext_data: Vec<EldExtData>,
}


impl DynamicSizedBitsPacket for ELDSpecificConfig {
    fn get_packet_bits_count(&self) -> usize {
        1 + // frameLengthFlag
        1 + // aacSectionDataResilienceFlag
        1 + // aacScalefactorDataResilienceFlag
        1 + // aacSpectralDataResilienceFlag
        1 + // ldSbrPresentFlag
        self.ld_sbr.as_ref().map_or(0, |item| item.get_packet_bits_count()) +
        self.eld_ext_data.iter().fold(0, |prev, item| prev + item.get_packet_bits_count()) +
        4 // ELDEXT_TERM
    }
}

#[derive(Debug, Clone)]
pub struct EldExtData {
    pub eld_ext_type: u8,    // 4 bits
    pub other_byte: Vec<u8>, // 8 bits
}

impl DynamicSizedBitsPacket for EldExtData {
    fn get_packet_bits_count(&self) -> usize {
        let len = self.other_byte.len();
        let result = len * 8 + 4; // eldExtType
        if len < 15 {
            return result + 4; // eldExtLen
        }
        if len < 255 + 15 {
            return result + 
              4 + // eldExtLen
              8; // eldExtLenAdd
        }
        if len >= 15 + 255 + u16::MAX.to_usize().unwrap() {
            panic!("len too large")
        }
        result +
          4 + // eldExtLen
          8 + // eldExtLenAdd
          16 // eldExtLenAddAdd
    }
}

#[derive(Debug, Clone)]
pub struct LdSbr {
    pub ld_sbr_sampling_rate: bool, // 1 bit
    pub ld_sbr_crc_flag: bool,      // 1 bit
    pub ld_sbr_header: LdSbrHeader,
}

impl DynamicSizedBitsPacket for LdSbr {
    fn get_packet_bits_count(&self) -> usize {
        1 + // ldSbrSamplingRate
        1 + // ldSbrCrcFlag
        self.ld_sbr_header.get_packet_bits_count()
    }
}

#[derive(Debug, Clone)]
pub struct LdSbrHeader {
    #[allow(unused)]
    num_sbr_header: usize,
    pub sbr_headers: Vec<SbrHeader>,
}

impl DynamicSizedBitsPacket for LdSbrHeader {
    fn get_packet_bits_count(&self) -> usize {
        self.sbr_headers
            .iter()
            .fold(0, |prev, item| prev + item.get_packet_bits_count())
    }
}
