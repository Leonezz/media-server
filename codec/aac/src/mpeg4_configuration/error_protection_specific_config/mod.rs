use utils::traits::dynamic_sized_packet::DynamicSizedBitsPacket;

pub mod reader;
pub mod writer;
/// @see: Table 1.49 – Syntax of ErrorProtectionSpecificConfig()
#[derive(Debug, Clone)]
pub struct ErrorProtectionSpecificConfig {
    #[allow(unused)]
    number_of_predefined_set: u8, // 8 bits
    pub interleave_type: u8,              // 2 bits
    pub bit_stuffing: u8,                 // 3 bits
    pub number_of_concatenated_frame: u8, // 3 bits
    pub predefined_sets: Vec<PredefinedSet>,
    pub header_protection: bool,   // 1 bit
    pub header_rate: Option<u8>,   // 5 bits
    pub header_crclen: Option<u8>, // 5 bits
}

impl DynamicSizedBitsPacket for ErrorProtectionSpecificConfig {
    fn get_packet_bits_count(&self) -> usize {
        8 + // number_of_predefined_set
        2 + // interleave_type
        3 + // bit_stuffing
        3 + // number_of_concatenated_frame
        self.predefined_sets.iter().fold(0, |prev, item| prev + item.get_packet_bits_count()) +
        1 + // header_protection
        self.header_rate.map_or(0, |_| 5) +
        self.header_crclen.map_or(0, |_| 5)
    }
}

#[derive(Debug, Clone)]
pub struct PredefinedSet {
    #[allow(unused)]
    number_of_class: u8, // 6 bis
    pub class: Vec<PredefinedSetClass>,
    #[allow(unused)]
    class_reordered_output: bool, // 1 bit
    pub class_output_order: Option<Vec<u8>>, // 6 bits
}

impl DynamicSizedBitsPacket for PredefinedSet {
    fn get_packet_bits_count(&self) -> usize {
        6 + // number_of_class
        self.class.iter().fold(0, |prev, item| prev + item.get_packet_bits_count()) +
        1 + // class_reordered_output
        self.class_output_order.as_ref().map_or(0, |item| item.len() * 6)
    }
}

#[derive(Debug, Clone)]
pub struct PredefinedSetClass {
    pub length_escape: bool,                   // 1 bit
    pub rate_escape: bool,                     // 1 bit
    pub crclen_escape: bool,                   // 1 bit
    pub concatenate_flag: Option<bool>,        // 1 bit
    pub fec_type: u8,                          // 2 bits
    pub termination_switch: Option<bool>,      // 1 bit
    pub interleave_switch: Option<u8>,         // 2 bits
    pub class_optional: bool,                  // 1 bit
    pub number_of_bits_for_length: Option<u8>, // 4 bits
    pub class_length: Option<u16>,             // 16 bits
    pub class_rate_7bits: Option<u8>,          // 7 bits
    pub class_rate_5bits: Option<u8>,          // 5 bits
    pub class_crclen: Option<u8>,              // 5 bits
}

impl DynamicSizedBitsPacket for PredefinedSetClass {
    fn get_packet_bits_count(&self) -> usize {
        1 + // length_escape
        1 + // rate_escape
        1 + // crclen_escape
        self.concatenate_flag.map_or(0, |_| 1) +
        2 + // fec_type
        self.termination_switch.map_or(0, |_| 1) +
        self.interleave_switch.map_or(0, |_| 2) +
        1 + // class_optional
        self.number_of_bits_for_length.map_or(0, |_| 4) +
        self.class_length.map_or(0, |_| 16) +
        self.class_rate_7bits.map_or(0, |_| 7) +
        self.class_rate_5bits.map_or(0, |_| 5) +
        self.class_crclen.map_or(0, |_| 5)
    }
}
