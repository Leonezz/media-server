//!! @see: Table 1.18 – Sampling Frequency Index

use crate::errors::AACCodecError;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingFrequencyIndex {
    F96000 = 0x0,
    F88200 = 0x1,
    F64000 = 0x2,
    F48000 = 0x3,
    F44100 = 0x4,
    F32000 = 0x5,
    F24000 = 0x6,
    F22050 = 0x7,
    F16000 = 0x8,
    F12000 = 0x9,
    F11025 = 0xa,
    F8000 = 0xb,
    F7350 = 0xc,
    Reserved(u8), // 0xd, 0xe
    Escape = 0xf,
}

impl SamplingFrequencyIndex {
    pub fn get_sampling_frequency(&self) -> Option<u32> {
        match self {
            SamplingFrequencyIndex::F96000 => Some(96000),
            SamplingFrequencyIndex::F88200 => Some(88200),
            SamplingFrequencyIndex::F64000 => Some(64000),
            SamplingFrequencyIndex::F48000 => Some(48000),
            SamplingFrequencyIndex::F44100 => Some(44100),
            SamplingFrequencyIndex::F32000 => Some(32000),
            SamplingFrequencyIndex::F24000 => Some(24000),
            SamplingFrequencyIndex::F22050 => Some(22050),
            SamplingFrequencyIndex::F16000 => Some(16000),
            SamplingFrequencyIndex::F12000 => Some(12000),
            SamplingFrequencyIndex::F11025 => Some(11025),
            SamplingFrequencyIndex::F8000 => Some(8000),
            SamplingFrequencyIndex::F7350 => Some(7350),
            SamplingFrequencyIndex::Reserved(_) | SamplingFrequencyIndex::Escape => None,
        }
    }

    pub fn get_sampling_frequency_index(frequency: u32) -> Option<Self> {
        match frequency {
            96000 => Some(SamplingFrequencyIndex::F96000),
            88200 => Some(SamplingFrequencyIndex::F88200),
            64000 => Some(SamplingFrequencyIndex::F64000),
            48000 => Some(SamplingFrequencyIndex::F48000),
            44100 => Some(SamplingFrequencyIndex::F44100),
            32000 => Some(SamplingFrequencyIndex::F32000),
            24000 => Some(SamplingFrequencyIndex::F24000),
            22050 => Some(SamplingFrequencyIndex::F22050),
            16000 => Some(SamplingFrequencyIndex::F16000),
            12000 => Some(SamplingFrequencyIndex::F12000),
            11025 => Some(SamplingFrequencyIndex::F11025),
            8000 => Some(SamplingFrequencyIndex::F8000),
            7350 => Some(SamplingFrequencyIndex::F7350),
            _ => None,
        }
    }
}

impl From<SamplingFrequencyIndex> for u8 {
    fn from(value: SamplingFrequencyIndex) -> Self {
        match value {
            SamplingFrequencyIndex::F96000 => 0x0,
            SamplingFrequencyIndex::F88200 => 0x1,
            SamplingFrequencyIndex::F64000 => 0x2,
            SamplingFrequencyIndex::F48000 => 0x3,
            SamplingFrequencyIndex::F44100 => 0x4,
            SamplingFrequencyIndex::F32000 => 0x5,
            SamplingFrequencyIndex::F24000 => 0x6,
            SamplingFrequencyIndex::F22050 => 0x7,
            SamplingFrequencyIndex::F16000 => 0x8,
            SamplingFrequencyIndex::F12000 => 0x9,
            SamplingFrequencyIndex::F11025 => 0xa,
            SamplingFrequencyIndex::F8000 => 0xb,
            SamplingFrequencyIndex::F7350 => 0xc,
            SamplingFrequencyIndex::Reserved(v) => v,
            SamplingFrequencyIndex::Escape => 0xf,
        }
    }
}

impl TryFrom<u8> for SamplingFrequencyIndex {
    type Error = AACCodecError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x0 => Ok(SamplingFrequencyIndex::F96000),
            0x1 => Ok(SamplingFrequencyIndex::F88200),
            0x2 => Ok(SamplingFrequencyIndex::F64000),
            0x3 => Ok(SamplingFrequencyIndex::F48000),
            0x4 => Ok(SamplingFrequencyIndex::F44100),
            0x5 => Ok(SamplingFrequencyIndex::F32000),
            0x6 => Ok(SamplingFrequencyIndex::F24000),
            0x7 => Ok(SamplingFrequencyIndex::F22050),
            0x8 => Ok(SamplingFrequencyIndex::F16000),
            0x9 => Ok(SamplingFrequencyIndex::F12000),
            0xa => Ok(SamplingFrequencyIndex::F11025),
            0xb => Ok(SamplingFrequencyIndex::F8000),
            0xc => Ok(SamplingFrequencyIndex::F7350),
            0xd | 0xe => Ok(SamplingFrequencyIndex::Reserved(value)),
            0xf => Ok(SamplingFrequencyIndex::Escape),
            _ => Err(AACCodecError::UnknownAACSamplingFrequencyIndex(value)),
        }
    }
}
