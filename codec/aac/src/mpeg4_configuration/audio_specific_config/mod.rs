//!! @see: Information technology - Coding of  audio-visual objects Part 3:  Audio

use audio_object_type::AudioObjectType;
use sampling_frequency_index::SamplingFrequencyIndex;
use utils::traits::{
    dynamic_sized_packet::DynamicSizedBitsPacket, fixed_packet::FixedBitwisePacket,
};

use super::{
    als_specific_config::ALSSpecificConfig, celp_specific_config::CelpSpecificConfig,
    dst_specific_config::DSTSpecificConfig, eld_specific_config::ELDSpecificConfig,
    error_protection_specific_config::ErrorProtectionSpecificConfig,
    error_resilient_celp_specific_config::ErrorResilientCelpSpecificConfig,
    error_resilient_hvxc_specific_config::ErrorResilientHvxcSpecificConfig,
    ga_specific_config::GASpecificConfig, hvxc_specific_config::HvxcSpecificConfig,
    sls_specific_config::SLSSpecificConfig, ssc_specific_config::SSCSpecificConfig,
    structured_audio_specific_config::StructuredAudioSpecificConfig,
    tts_specific_config::TTSSpecificConfig,
};

pub mod audio_object_type;
pub mod reader;
pub mod sampling_frequency_index;
pub mod writer;

#[derive(Debug, Clone)]
pub struct AudioSpecificConfig {
    pub audio_object_type: AudioObjectType,
    pub sampling_frequency_index: SamplingFrequencyIndex, // 4 bits
    pub sampling_frequency: Option<u32>,                  // 24 bits
    pub channel_configuration: u8,                        // 4 bits
    pub sbr_present_flag: i32,
    pub ps_present_flag: i32,
    pub extension_audio_object_type: AudioObjectType,
    pub extension_sampling_frequency_index: Option<SamplingFrequencyIndex>, // 4 bits
    pub extension_sampling_frequency: Option<u32>,                          // 24 bits
    pub extension_channel_configuration: Option<u8>,                        // 4 bits
    pub specific_config: SpecificConfig,
    pub ep_config: Option<EpConfig>,
    pub sync_extension_type: Option<u16>, // 11 bits
    pub sync_extension_audio_object_type: Option<AudioObjectType>,
    pub sync_extension_audio_object_type5: Option<SyncExtensionAudioObjectType5>,
    pub sync_extension_audio_object_type22: Option<SyncExtensionAudioObjectType22>,
}

impl DynamicSizedBitsPacket for AudioSpecificConfig {
    fn get_packet_bits_count(&self) -> usize {
        self.audio_object_type.get_packet_bits_count() +
        4 + // samplingFrequencyIndex
        self.sampling_frequency.map_or(0, |_| 24) +
        4 + // channelConfiguration
        self.extension_sampling_frequency_index.map_or(0, |_| 4) +
        self.extension_sampling_frequency.map_or(0, |_| 24) +
        self.extension_audio_object_type.get_packet_bits_count() +
        self.extension_channel_configuration.map_or(0, |_| 4) +
        self.specific_config.get_packet_bits_count() +
        self.ep_config.as_ref().map_or(0, |item| item.get_packet_bits_count()) +
        self.sync_extension_type.map_or(0, |_| 11) +
        self.sync_extension_audio_object_type.map_or(0, |item| item.get_packet_bits_count()) +
        self.sync_extension_audio_object_type5.as_ref().map_or(0, |item| item.get_packet_bits_count()) +
        self.sync_extension_audio_object_type22.as_ref().map_or(0, |item| item.get_packet_bits_count())
    }
}

#[derive(Debug, Clone)]
pub enum SpecificConfig {
    Ga(GASpecificConfig),
    Celp(CelpSpecificConfig),
    Hvxc(HvxcSpecificConfig),
    TTS(TTSSpecificConfig),
    StructuredAudio(StructuredAudioSpecificConfig),
    ErrorResilientCelp(ErrorResilientCelpSpecificConfig),
    ErrorResilientHvxc(ErrorResilientHvxcSpecificConfig),
    Parametric(/*TODO */),
    SSC(SSCSpecificConfig),
    Spatial {
        // TODO
    },
    Mpeg1_2 {
        extension: bool, // 1 bit
    },
    DST(DSTSpecificConfig),
    ALS {
        fill_bits: u8, // 5 bits
        config: ALSSpecificConfig,
    },
    SLS(SLSSpecificConfig),
    ELD(ELDSpecificConfig),
    SymbolicMusic(/*TODO */),
    Reserved,
}

impl DynamicSizedBitsPacket for SpecificConfig {
    fn get_packet_bits_count(&self) -> usize {
        match self {
            Self::Ga(ga) => ga.get_packet_bits_count(),
            Self::Celp(celp) => celp.get_packet_bits_count(),
            Self::Hvxc(hvxc) => hvxc.get_packet_bits_count(),
            Self::TTS(_) => TTSSpecificConfig::bits_count(),
            Self::StructuredAudio(structure_audio) => structure_audio.get_packet_bits_count(),
            Self::ErrorResilientCelp(error_celp) => error_celp.get_packet_bits_count(),
            Self::ErrorResilientHvxc(error_hvxc) => error_hvxc.get_packet_bits_count(),
            Self::Parametric() => {
                unimplemented!()
            }
            Self::SSC(ssc) => ssc.get_packet_bits_count(),
            Self::Spatial {} => unimplemented!(),
            Self::Mpeg1_2 { .. } => 1,
            Self::DST(_) => DSTSpecificConfig::bits_count(),
            Self::ALS {
                fill_bits: _,
                config,
            } => {
                5 + // fillBits
                config.get_packet_bits_count()
            }
            Self::SLS(sls) => sls.get_packet_bits_count(),
            Self::ELD(eld) => eld.get_packet_bits_count(),
            Self::SymbolicMusic() => unimplemented!(),
            Self::Reserved => 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EpConfig {
    pub ep_config: u8, // 2
    // if ep_config == 2 || ep_config == 3 {
    pub error_protection_specific_config: Option<ErrorProtectionSpecificConfig>,
    // }
    // if ep_config == 3 {
    pub direct_mapping: Option<bool>, // 1 bit
                                      //   if direct_mapping {
                                      //      TBD
                                      //   }
                                      //}
}

impl DynamicSizedBitsPacket for EpConfig {
    fn get_packet_bits_count(&self) -> usize {
        2 + // epConfig
        self.error_protection_specific_config.as_ref().map_or(0, |item| item.get_packet_bits_count()) +
        self.direct_mapping.map_or(0, |_| 1)
    }
}

#[derive(Debug, Clone)]
pub struct SyncExtensionAudioObjectType5 {
    pub sbr_present_flag: bool, // 1 bit
    pub extension_sampling_frequency_index: Option<SamplingFrequencyIndex>, // 4 bits
    pub extension_sampling_frequency: Option<u32>, // 24 bits
    pub sync_extension_type: Option<u16>, // 11 bits
    pub ps_present_flag: Option<bool>, // 1 bit
}

impl DynamicSizedBitsPacket for SyncExtensionAudioObjectType5 {
    fn get_packet_bits_count(&self) -> usize {
        1 + // sbrPresentFlag
        self.extension_sampling_frequency_index.map_or(0, |_| 4) +
        self.extension_sampling_frequency.map_or(0, |_| 24) +
        self.sync_extension_type.map_or(0, |_| 11) +
        self.ps_present_flag.map_or(0, |_| 1)
    }
}

#[derive(Debug, Clone)]
pub struct SyncExtensionAudioObjectType22 {
    pub sbr_present_flag: bool, // 1 bit
    pub extension_sampling_frequency_index: Option<SamplingFrequencyIndex>, // 4 bits
    pub extension_sampling_frequency: Option<u32>, // 24 bits
    pub extension_channel_configuration: u8, // 4 bits
}

impl DynamicSizedBitsPacket for SyncExtensionAudioObjectType22 {
    fn get_packet_bits_count(&self) -> usize {
        1 + // sbrPresentFlag
        self.extension_sampling_frequency_index.map_or(0, |_| 4) +
        self.extension_sampling_frequency.map_or(0, |_| 24) +
        4 // extensionChannelConfiguration
    }
}
