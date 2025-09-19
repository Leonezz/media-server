use utils::traits::{dynamic_sized_packet::DynamicSizedBitsPacket, fixed_packet::FixedBitwisePacket};

use super::Symbol;

#[derive(Debug, Clone)]
pub struct SampleLoop {
    pub loopstart: u32, // 24 bits
    pub loopend: u32,   // 24 bits
}

impl FixedBitwisePacket for SampleLoop {
    fn bits_count() -> usize {
        24 + // loopstart
        24 // loopend
    }
}

#[derive(Debug, Clone)]
pub struct Sample {
    pub sample_name_sym: Symbol,
    pub length: u32, // 24 bits
    #[allow(unused)]
    pub(crate) has_srate: bool, // 1 bit
    // if has_srate {
    pub srate: Option<u32>, // 17 bits
    //}
    #[allow(unused)]
    pub(crate) has_loop: bool, // 1 bit
    // if has_loop {
    pub sample_loop: Option<SampleLoop>,
    //}
    #[allow(unused)]
    pub(crate) has_base: bool, // 1 bit
    // if has_base {
    pub basecps: Option<f32>, // 32 bits
    //}
    pub float_sample: bool, // 1 bit
    // if float_sample {
    pub float_sample_data: Option<Vec<f32>>, // f32 float_sample_data[length]
    // } else {
    pub sample_data: Option<Vec<i16>>, // u16 sample_data[length]
                                       // }
}

impl DynamicSizedBitsPacket for Sample {
    fn get_packet_bits_count(&self) -> usize {
        16 + // sample_name_sym
        24 + // length
        1 + // has_srate
        self.srate.map_or(0, |_| 17) + 
        1 + // has_loop
        self.sample_loop.as_ref().map_or(0, |_| SampleLoop::bits_count()) +
        1 + // has_base
        self.basecps.map_or(0, |_|32) +
        1 + // float_sample
        self.float_sample_data.as_ref().map_or(0, |item| item.len() * 32) +
        self.sample_data.as_ref().map_or(0, |item| item.len() * 16)
    }
}