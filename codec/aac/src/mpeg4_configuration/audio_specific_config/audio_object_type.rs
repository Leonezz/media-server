//!!! @see: Table 1.1 – Audio Object Type definition based on Tools/Modules

use utils::traits::dynamic_sized_packet::DynamicSizedBitsPacket;

use crate::errors::AACCodecError;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioObjectType {
    NULL = 0,
    AACMain = 1,
    AACLC = 2,
    AACSSR = 3,
    AACLTP = 4,
    SBR = 5,
    AACScalable = 6,
    TwinVQ = 7,
    CELP = 8,
    HVXC = 9,
    Reserved(u8), // 10, 11, 18, 42~95
    TTSI = 12,
    MainSynthetic = 13,
    WavetableSynthesis = 14,
    GeneralMIDI = 15,
    AlgorithmicSynthesisAndAudioFX = 16,
    ERAACLC = 17,
    ERAACLTP = 19,
    ERAACScalable = 20,
    ERTwinVQ = 21,
    ERBSAC = 22,
    ERAACLD = 23,
    ERCELP = 24,
    ERHVXC = 25,
    ERHILN = 26,
    ERParametric = 27,
    SSC = 28,
    PS = 29,
    MPEGSurround = 30,
    Escape = 31,
    Layer1 = 32,
    Layer2 = 33,
    Layer3 = 34,
    DST = 35,
    ALS = 36,
    SLS = 37,
    SLSNonCore = 38,
    ERAACELD = 39,
    SMRSimple = 40,
    SMRMain = 41,
}

impl DynamicSizedBitsPacket for AudioObjectType {
    fn get_packet_bits_count(&self) -> usize {
        let value: u8 = (*self).into();
        if value < 31 {
            return 5;
        }
        11
    }
}

impl From<AudioObjectType> for u8 {
    fn from(value: AudioObjectType) -> Self {
        match value {
            AudioObjectType::NULL => 0,
            AudioObjectType::AACMain => 1,
            AudioObjectType::AACLC => 2,
            AudioObjectType::AACSSR => 3,
            AudioObjectType::AACLTP => 4,
            AudioObjectType::SBR => 5,
            AudioObjectType::AACScalable => 6,
            AudioObjectType::TwinVQ => 7,
            AudioObjectType::CELP => 8,
            AudioObjectType::HVXC => 9,
            AudioObjectType::Reserved(value) => value,
            AudioObjectType::TTSI => 12,
            AudioObjectType::MainSynthetic => 13,
            AudioObjectType::WavetableSynthesis => 14,
            AudioObjectType::GeneralMIDI => 15,
            AudioObjectType::AlgorithmicSynthesisAndAudioFX => 16,
            AudioObjectType::ERAACLC => 17,
            AudioObjectType::ERAACLTP => 19,
            AudioObjectType::ERAACScalable => 20,
            AudioObjectType::ERTwinVQ => 21,
            AudioObjectType::ERBSAC => 22,
            AudioObjectType::ERAACLD => 23,
            AudioObjectType::ERCELP => 24,
            AudioObjectType::ERHVXC => 25,
            AudioObjectType::ERHILN => 26,
            AudioObjectType::ERParametric => 27,
            AudioObjectType::SSC => 28,
            AudioObjectType::PS => 29,
            AudioObjectType::MPEGSurround => 30,
            AudioObjectType::Escape => 31,
            AudioObjectType::Layer1 => 32,
            AudioObjectType::Layer2 => 33,
            AudioObjectType::Layer3 => 34,
            AudioObjectType::DST => 35,
            AudioObjectType::ALS => 36,
            AudioObjectType::SLS => 37,
            AudioObjectType::SLSNonCore => 38,
            AudioObjectType::ERAACELD => 39,
            AudioObjectType::SMRSimple => 40,
            AudioObjectType::SMRMain => 41,
        }
    }
}

impl TryFrom<u8> for AudioObjectType {
    type Error = AACCodecError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(AudioObjectType::NULL),
            1 => Ok(AudioObjectType::AACMain),
            2 => Ok(AudioObjectType::AACLC),
            3 => Ok(AudioObjectType::AACSSR),
            4 => Ok(AudioObjectType::AACLTP),
            5 => Ok(AudioObjectType::SBR),
            6 => Ok(AudioObjectType::AACScalable),
            7 => Ok(AudioObjectType::TwinVQ),
            8 => Ok(AudioObjectType::CELP),
            9 => Ok(AudioObjectType::HVXC),
            12 => Ok(AudioObjectType::TTSI),
            13 => Ok(AudioObjectType::MainSynthetic),
            14 => Ok(AudioObjectType::WavetableSynthesis),
            15 => Ok(AudioObjectType::GeneralMIDI),
            16 => Ok(AudioObjectType::AlgorithmicSynthesisAndAudioFX),
            17 => Ok(AudioObjectType::ERAACLC),
            19 => Ok(AudioObjectType::ERAACLTP),
            20 => Ok(AudioObjectType::ERAACScalable),
            21 => Ok(AudioObjectType::ERTwinVQ),
            22 => Ok(AudioObjectType::ERBSAC),
            23 => Ok(AudioObjectType::ERAACLD),
            24 => Ok(AudioObjectType::ERCELP),
            25 => Ok(AudioObjectType::ERHVXC),
            26 => Ok(AudioObjectType::ERHILN),
            27 => Ok(AudioObjectType::ERParametric),
            28 => Ok(AudioObjectType::SSC),
            29 => Ok(AudioObjectType::PS),
            30 => Ok(AudioObjectType::MPEGSurround),
            31 => Ok(AudioObjectType::Escape),
            32 => Ok(AudioObjectType::Layer1),
            33 => Ok(AudioObjectType::Layer2),
            34 => Ok(AudioObjectType::Layer3),
            35 => Ok(AudioObjectType::DST),
            36 => Ok(AudioObjectType::ALS),
            37 => Ok(AudioObjectType::SLS),
            38 => Ok(AudioObjectType::SLSNonCore),
            39 => Ok(AudioObjectType::ERAACELD),
            40 => Ok(AudioObjectType::SMRSimple),
            41 => Ok(AudioObjectType::SMRMain),
            10..=11 | 18 | 42..=95 => Ok(AudioObjectType::Reserved(value)),
            _ => Err(AACCodecError::UnknownAACObjectTypeId(value)),
        }
    }
}
