use utils::traits::{
    dynamic_sized_packet::DynamicSizedBitsPacket, fixed_packet::FixedBitwisePacket,
};

pub mod reader;
pub mod writer;

#[derive(Debug, Clone, Copy)]
pub struct MPEExciationMode {
    pub mpe_configuration: u8,            // 5 bits
    pub num_enh_layers: u8,               // 2 bits
    pub bandwidth_scalability_mode: bool, // 1 bit
}

impl FixedBitwisePacket for MPEExciationMode {
    fn bits_count() -> usize {
        5 + // MPE_Configuration
        2 + // NumEnhLayers
        1 // BandwidthScalabilityMode
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExcitationMode {
    MPE = 0,
    RPE = 1,
}

impl From<ExcitationMode> for bool {
    fn from(value: ExcitationMode) -> Self {
        match value {
            ExcitationMode::MPE => false,
            ExcitationMode::RPE => true,
        }
    }
}

impl From<bool> for ExcitationMode {
    fn from(value: bool) -> Self {
        match value {
            true => Self::RPE,
            false => Self::MPE,
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleRateMode {
    KHz8 = 0,
    KHz16 = 1,
}

impl From<SampleRateMode> for bool {
    fn from(value: SampleRateMode) -> Self {
        match value {
            SampleRateMode::KHz16 => true,
            SampleRateMode::KHz8 => false,
        }
    }
}

impl From<bool> for SampleRateMode {
    fn from(value: bool) -> Self {
        match value {
            true => Self::KHz16,
            false => Self::KHz8,
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FineRateControl {
    OFF = 0,
    ON = 1,
}

impl From<FineRateControl> for bool {
    fn from(value: FineRateControl) -> Self {
        match value {
            FineRateControl::OFF => false,
            FineRateControl::ON => true,
        }
    }
}

impl From<bool> for FineRateControl {
    fn from(value: bool) -> Self {
        match value {
            true => Self::ON,
            false => Self::OFF,
        }
    }
}

/// @see: Table 3.11 — Syntax of CelpHeader()
#[derive(Debug, Clone)]
pub struct CelpHeader {
    pub excitation_mode: ExcitationMode,    // 1 bit
    pub sample_rate_mode: SampleRateMode,   // 1 bit
    pub fine_rate_control: FineRateControl, // 1 bit
    pub rpe_configuration: Option<u8>,      // 3 bits
    pub mpe_exciation_mode: Option<MPEExciationMode>,
}

impl DynamicSizedBitsPacket for CelpHeader {
    fn get_packet_bits_count(&self) -> usize {
        1 + // ExcitationMode
        1 + // SampleRateMode
        1 + // FineRateControl
        self.rpe_configuration.map_or(0, |_| 3) +
        self.mpe_exciation_mode.as_ref().map_or(0, |_| MPEExciationMode::bits_count())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CelpBWSenhHeader {
    pub bws_configuration: u8, // 2 bits
}

impl FixedBitwisePacket for CelpBWSenhHeader {
    fn bits_count() -> usize {
        2
    }
}
