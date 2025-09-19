use utils::traits::{
    dynamic_sized_packet::DynamicSizedBitsPacket, fixed_packet::FixedBitwisePacket,
};

use super::{
    audio_specific_config::sampling_frequency_index::SamplingFrequencyIndex,
    celp_header::{
        CelpBWSenhHeader, ExcitationMode, FineRateControl, MPEExciationMode, SampleRateMode,
    },
};
pub mod reader;
pub mod writer;

/// @see: Table 3.24 — Syntax of ER_SC_CelpHeader()
#[derive(Debug, Clone)]
pub struct ErScCelpHeader {
    pub excitation_mode: ExcitationMode,    // 1 bit
    pub sample_rate_mode: SampleRateMode,   // 1 bit
    pub fine_rate_control: FineRateControl, // 1 bit
    pub silence_compression: bool,          // 1 bit
    // if excitation_mode == RPE {
    pub rpe_configuration: Option<u8>, // 3 bits
    // }
    // if excitation_mode == MPE {
    pub excitation_mode_mpe: Option<MPEExciationMode>,
    // }
    pub sampling_frequency_index: SamplingFrequencyIndex,
}

impl DynamicSizedBitsPacket for ErScCelpHeader {
    fn get_packet_bits_count(&self) -> usize {
        1 + // ExcitationMode
        1 + // SampleRateMode
        1 + // FineRateControl
        1 + // SilenceCompression
        self.rpe_configuration.map_or(0, |_|3) +
        self.excitation_mode_mpe.as_ref().map_or(0, |_| MPEExciationMode::bits_count())
    }
}

/// @see: Table 3.23 — Syntax of ErrorResilientCelpSpecificConfig ()
#[derive(Debug, Clone)]
pub struct ErrorResilientCelpSpecificConfig {
    pub is_base_layer: bool, // 1 bit
    // if is_base_layer {
    pub er_sc_celp_header: Option<ErScCelpHeader>,
    // } else {
    pub is_bws_layer: Option<bool>, // 1 bit
    //   if is_bws_layer {
    pub celp_bw_senh_header: Option<CelpBWSenhHeader>,
    //   } else {
    pub celp_brs_id: Option<u8>, // 2 bits
                                 //   }
                                 // }
}

impl DynamicSizedBitsPacket for ErrorResilientCelpSpecificConfig {
    fn get_packet_bits_count(&self) -> usize {
        1 + // is_base_layer
        self.er_sc_celp_header.as_ref().map_or(0, |item| item.get_packet_bits_count()) +
        self.is_bws_layer.map_or(0, |_| 1) +
        self.celp_bw_senh_header.map_or(0, |_| CelpBWSenhHeader::bits_count()) +
        self.celp_brs_id.map_or(0, |_| 2)
    }
}
