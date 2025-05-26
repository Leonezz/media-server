use utils::traits::dynamic_sized_packet::DynamicSizedBitsPacket;

use crate::exp_golomb::find_ue_bits_count;

#[derive(Debug, Clone)]
pub struct SchedSel {
    pub bit_rate_value_minus1: u64, // ue(v), in [0, 2^32 - 2]. bit_rate_value_minus1[SchedSelIdx] shall be greater than bit_rate_value_minus1[SchedSelIdx − 1]
    pub cpb_size_value_minus1: u64, // ue(v), in [0, 2^32 - 2]. cpb_size_value_minus1[SchedSelIdx] shall be less than or equal to cpb_size_value_minus1[SchedSelIdx − 1]
    pub cbr_flag: bool,             // u(1)
}

impl DynamicSizedBitsPacket for SchedSel {
    fn get_packet_bits_count(&self) -> usize {
        find_ue_bits_count(self.bit_rate_value_minus1).unwrap() +
        find_ue_bits_count(self.cpb_size_value_minus1).unwrap() +
        1 // cbr_flag
    }
}

#[derive(Debug, Clone)]
pub struct HrdParameters {
    #[allow(unused)]
    pub(crate) cpb_cnt_minus1: u8, // ue(v), in [0, 31]
    pub bit_rate_scale: u8, // u(4)
    pub cpb_size_scale: u8, // u(4)
    /// for( SchedSelIdx = 0; SchedSelIdx <= cpb_cnt_minus1; SchedSelIdx++ ) {
    pub sched_sels: Vec<SchedSel>,
    /// }
    pub initial_cpb_removal_delay_length_minus1: u8, // u(5)
    pub cpb_removal_delay_length_minus1: u8, // u(5)
    pub dpb_output_delay_length_minus1: u8,  // u(5)
    pub time_offset_length: u8,              // u(5)
}

impl DynamicSizedBitsPacket for HrdParameters {
    fn get_packet_bits_count(&self) -> usize {
        find_ue_bits_count(self.cpb_cnt_minus1).unwrap() + 
        4 + // bit_rate_scale
        4 + // cpb_size_scale
        self.sched_sels.iter().fold(0, |prev, item| prev + item.get_packet_bits_count()) +
        5 + // initial_cpb_removal_delay_length_minus1
        5 + // cpb_removal_delay_length_minus1
        5 + // dpb_output_delay_length_minus1
        5 // time_offset_length
    }
}