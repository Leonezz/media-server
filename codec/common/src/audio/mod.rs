use crate::FrameType;
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SoundRateCommon {
    KHZ5D5,
    KHZ11,
    KHZ22,
    KHZ44,
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
