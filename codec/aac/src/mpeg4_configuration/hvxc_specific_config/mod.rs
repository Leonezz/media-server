use utils::traits::{
    dynamic_sized_packet::DynamicSizedBitsPacket, fixed_packet::FixedBitwisePacket,
};

use crate::errors::AACCodecError;
pub mod reader;
pub mod writer;
/// @see: 2.3.1Decoder configuration (HvxcSpecificConfig)
#[derive(Debug, Clone)]
pub struct HvxcSpecificConfig {
    #[allow(unused)]
    is_base_layer: bool, // 1 bit
    // if is_base_layer {
    pub config: Option<HVXCConfig>,
    // }
}

impl DynamicSizedBitsPacket for HvxcSpecificConfig {
    fn get_packet_bits_count(&self) -> usize {
        1 + // isBaseLayer
        self.config.as_ref().map_or(0, |_| HVXCConfig::bits_count())
    }
}

/// @see: Table 2.2 — HVXCvarMode
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum HVXCvarMode {
    FixedBitRate = 0,
    VariableBitRate = 1,
}

impl From<HVXCvarMode> for bool {
    fn from(value: HVXCvarMode) -> Self {
        match value {
            HVXCvarMode::FixedBitRate => false,
            HVXCvarMode::VariableBitRate => true,
        }
    }
}

impl From<bool> for HVXCvarMode {
    fn from(value: bool) -> Self {
        match value {
            true => Self::VariableBitRate,
            false => Self::FixedBitRate,
        }
    }
}

/// @see: Table 2.3 — HVXCrateMode
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum HVXCrateMode {
    HVXC2Kbps = 0,
    HVXC4Kbps = 1,
    HVXC3p7Kbps = 2,
    Reserved = 3,
}

impl From<HVXCrateMode> for u8 {
    fn from(value: HVXCrateMode) -> Self {
        match value {
            HVXCrateMode::HVXC2Kbps => 0,
            HVXCrateMode::HVXC4Kbps => 1,
            HVXCrateMode::HVXC3p7Kbps => 2,
            HVXCrateMode::Reserved => 3,
        }
    }
}

impl TryFrom<u8> for HVXCrateMode {
    type Error = AACCodecError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::HVXC2Kbps),
            1 => Ok(Self::HVXC4Kbps),
            2 => Ok(Self::HVXC3p7Kbps),
            3 => Ok(Self::Reserved),
            _ => Err(AACCodecError::UnknownHVXCrateMode(value)),
        }
    }
}

/// @see: Table 2.1 — Syntax of HVXCconfig()
#[derive(Debug, Clone)]
pub struct HVXCConfig {
    pub hvxc_var_mode: HVXCvarMode,   // 1 bit
    pub hvxc_rate_mode: HVXCrateMode, // 2 bits
    pub extension_flag: bool,         // 1 bit
                                      // if extension_flag {
                                      //   < to be defined in MPEG-4 Version 2 >
                                      // }
}

impl FixedBitwisePacket for HVXCConfig {
    fn bits_count() -> usize {
        1 + // HVXCvarMode
        2 + // HVXCrateMode
        1 // extensionFlag
    }
}
