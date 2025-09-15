use crate::{FrameType, errors::CodecCommonError};
pub mod reader;
pub mod writer;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioCodecCommon {
    LinearPCM,
    ADPCM,
    MP3,
    LinearPCMLittleEndian,
    NellyMoser16KHZ,
    NellyMoser8KHZ,
    NellyMoser,
    G711ALawLogarithmicPCM,  // reserved
    G711MULawLogarithmicPCM, // reserved
    AAC,
    Speex,
    MP38KHZ, // reserved,
    DeviceSpecific,
    AC3,
    EAC3,
    OPUS,
    FLAC,
}

impl AudioCodecCommon {
    pub fn get_codec_name(&self) -> &'static str {
        match self {
            Self::LinearPCM => "LinearPCM",
            Self::ADPCM => "ADPCM",
            Self::MP3 => "MP3",
            Self::LinearPCMLittleEndian => "LinearPCMLittleEndian",
            Self::NellyMoser16KHZ => "NellyMoser16KHZ",
            Self::NellyMoser8KHZ => "NellyMoser8KHZ",
            Self::NellyMoser => "NellyMoser",
            Self::G711ALawLogarithmicPCM => "G711ALowLogarithmicPCM",
            Self::G711MULawLogarithmicPCM => "G711MULawLogarithmicPCM",
            Self::AAC => "AAC",
            Self::Speex => "Speex",
            Self::MP38KHZ => "MP38KHZ",
            Self::DeviceSpecific => "DeviceSpecific",
            Self::AC3 => "AC3",
            Self::EAC3 => "EAC3",
            Self::OPUS => "OPUS",
            Self::FLAC => "FLAC",
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SoundRateCommon {
    KHZ5D5,
    KHZ11,
    KHZ22,
    KHZ44,
}
type AACSamplingFrequencyIndex = codec_aac::mpeg4_configuration::audio_specific_config::sampling_frequency_index::SamplingFrequencyIndex;
impl TryFrom<AACSamplingFrequencyIndex> for SoundRateCommon {
    type Error = CodecCommonError;
    fn try_from(value: AACSamplingFrequencyIndex) -> Result<Self, Self::Error> {
        match value {
            AACSamplingFrequencyIndex::F96000 => Ok(Self::KHZ44),
            AACSamplingFrequencyIndex::F88200 => Ok(Self::KHZ44),
            AACSamplingFrequencyIndex::F64000 => Ok(Self::KHZ44),
            AACSamplingFrequencyIndex::F48000 => Ok(Self::KHZ44),
            AACSamplingFrequencyIndex::F44100 => Ok(Self::KHZ44),
            AACSamplingFrequencyIndex::F32000 => Ok(Self::KHZ22),
            AACSamplingFrequencyIndex::F24000 => Ok(Self::KHZ22),
            AACSamplingFrequencyIndex::F22050 => Ok(Self::KHZ22),
            AACSamplingFrequencyIndex::F16000 => Ok(Self::KHZ11),
            AACSamplingFrequencyIndex::F12000 => Ok(Self::KHZ11),
            AACSamplingFrequencyIndex::F11025 => Ok(Self::KHZ11),
            AACSamplingFrequencyIndex::F8000 => Ok(Self::KHZ5D5),
            AACSamplingFrequencyIndex::F7350 => Ok(Self::KHZ5D5),
            AACSamplingFrequencyIndex::Reserved(v) => {
                Err(CodecCommonError::InvalidSamplingFrequencyIndex(v))
            }
            AACSamplingFrequencyIndex::Escape => {
                Err(CodecCommonError::InvalidSamplingFrequencyIndex(0xF))
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SoundSizeCommon {
    Bit8,
    Bit16,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SoundTypeCommon {
    Mono,
    Stereo,
}

#[derive(Debug, Clone, Copy)]
pub struct SoundInfoCommon {
    pub sound_rate: SoundRateCommon,
    pub sound_size: SoundSizeCommon,
    pub sound_type: SoundTypeCommon,
}

impl TryFrom<&codec_aac::mpeg4_configuration::audio_specific_config::AudioSpecificConfig>
    for SoundInfoCommon
{
    type Error = CodecCommonError;
    fn try_from(
        value: &codec_aac::mpeg4_configuration::audio_specific_config::AudioSpecificConfig,
    ) -> Result<Self, Self::Error> {
        let sound_rate: SoundRateCommon = value.sampling_frequency_index.try_into()?;
        let sound_type = if value.channel_configuration == 1 {
            SoundTypeCommon::Mono
        } else if value.channel_configuration == 0 {
            tracing::warn!("channel configuration is 0, treat as stereo");
            SoundTypeCommon::Stereo
        } else {
            SoundTypeCommon::Stereo
        };
        Ok(Self {
            sound_rate,
            sound_size: SoundSizeCommon::Bit16,
            sound_type,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AudioFrameInfo {
    pub codec_id: AudioCodecCommon,
    pub frame_type: FrameType,
    pub sound_info: SoundInfoCommon,
    pub timestamp_nano: u64,
}

impl AudioFrameInfo {
    pub fn new(
        codec_id: AudioCodecCommon,
        frame_type: FrameType,
        sound_rate: SoundRateCommon,
        sound_size: SoundSizeCommon,
        sound_type: SoundTypeCommon,
        timestamp_nano: u64,
    ) -> Self {
        Self {
            codec_id,
            frame_type,
            sound_info: SoundInfoCommon {
                sound_rate,
                sound_size,
                sound_type,
            },
            timestamp_nano,
        }
    }
}

#[derive(Debug, Clone)]
pub enum AudioConfig {
    AAC(codec_aac::mpeg4_configuration::audio_specific_config::AudioSpecificConfig),
}

impl From<codec_aac::mpeg4_configuration::audio_specific_config::AudioSpecificConfig>
    for AudioConfig
{
    fn from(
        value: codec_aac::mpeg4_configuration::audio_specific_config::AudioSpecificConfig,
    ) -> Self {
        Self::AAC(value)
    }
}

impl From<&AudioConfig> for AudioCodecCommon {
    fn from(value: &AudioConfig) -> Self {
        match value {
            AudioConfig::AAC(_) => Self::AAC,
        }
    }
}
