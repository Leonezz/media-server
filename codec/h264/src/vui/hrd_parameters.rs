#[derive(Debug)]
pub struct SchedSel {
    pub bit_rate_value_minus1: u64, // ue(v), in [0, 2^32 - 2]. bit_rate_value_minus1[SchedSelIdx] shall be greater than bit_rate_value_minus1[SchedSelIdx − 1]
    pub cpb_size_value_minus1: u64, // ue(v), in [0, 2^32 - 2]. cpb_size_value_minus1[SchedSelIdx] shall be less than or equal to cpb_size_value_minus1[SchedSelIdx − 1]
    pub cbr_flag: bool,             // u(1)
}

#[derive(Debug)]
pub struct HrdParameters {
    pub(crate) cpb_cnt_minus1: u8, // ue(v), in [0, 31]
    pub bit_rate_scale: u8,        // u(4)
    pub cpb_size_scale: u8,        // u(4)
    /// for( SchedSelIdx = 0; SchedSelIdx <= cpb_cnt_minus1; SchedSelIdx++ ) {
    pub sched_sels: Vec<SchedSel>,
    /// }
    pub initial_cpb_removal_delay_length_minus1: u8, // u(5)
    pub cpb_removal_delay_length_minus1: u8, // u(5)
    pub dpb_output_delay_length_minus1: u8,  // u(5)
    pub time_offset_length: u8,              // u(5)
}
