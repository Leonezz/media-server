use crate::{
    errors::AACCodecError,
    mpeg4_configuration::{
        als_specific_config::ALSSpecificConfig, celp_specific_config::CelpSpecificConfig,
        dst_specific_config::DSTSpecificConfig, eld_specific_config::ELDSpecificConfig,
        error_protection_specific_config::ErrorProtectionSpecificConfig,
        error_resilient_celp_specific_config::ErrorResilientCelpSpecificConfig,
        error_resilient_hvxc_specific_config::ErrorResilientHvxcSpecificConfig,
        ga_specific_config::GASpecificConfig, hvxc_specific_config::HvxcSpecificConfig,
        sls_specific_config::SLSSpecificConfig, ssc_specific_config::SSCSpecificConfig,
        structured_audio_specific_config::StructuredAudioSpecificConfig,
        tts_specific_config::TTSSpecificConfig,
    },
};

use super::{
    AudioSpecificConfig, EpConfig, SpecificConfig, SyncExtensionAudioObjectType5,
    SyncExtensionAudioObjectType22, audio_object_type::AudioObjectType,
    sampling_frequency_index::SamplingFrequencyIndex,
};

use bitstream_io::BitRead;
use codec_bitstream::reader::BitstreamReader;
use utils::traits::reader::{BitwiseReadFrom, BitwiseReadReaminingFrom};

impl<R: BitRead> BitwiseReadFrom<R> for AudioObjectType {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let mut audio_object_type = reader.read::<5, u8>()?;
        if audio_object_type == 31 {
            let audio_object_type_ext = reader.read::<6, u8>()?;
            audio_object_type = 32 + audio_object_type_ext;
        }
        audio_object_type.try_into()
    }
}

impl<'a> BitwiseReadFrom<BitstreamReader<'a>> for AudioSpecificConfig {
    type Error = AACCodecError;
    fn read_from(reader: &mut BitstreamReader<'a>) -> Result<Self, Self::Error> {
        let mut audio_object_type = AudioObjectType::read_from(reader)?;
        let sampling_frequency_index: SamplingFrequencyIndex =
            reader.read::<4, u8>()?.try_into()?;
        let sampling_frequency = if sampling_frequency_index == SamplingFrequencyIndex::Escape {
            Some(reader.read::<24, u32>()?)
        } else {
            None
        };
        let channel_configuration = reader.read::<4, u8>()?;
        let mut sbr_present_flag = -1;
        let mut ps_present_flag = -1;
        let mut extension_audio_object_type = AudioObjectType::NULL;
        let mut extension_sampling_frequency_index = None;
        let mut extension_sampling_frequency = None;
        let mut extension_channel_configuration = None;
        if audio_object_type == AudioObjectType::SBR || audio_object_type == AudioObjectType::PS {
            extension_audio_object_type = AudioObjectType::SBR;
            sbr_present_flag = 1;
            if audio_object_type == AudioObjectType::PS {
                ps_present_flag = 1;
            }
            extension_sampling_frequency_index =
                Some(SamplingFrequencyIndex::try_from(reader.read::<4, u8>()?)?);
            if extension_sampling_frequency_index.unwrap() == SamplingFrequencyIndex::Escape {
                extension_sampling_frequency = Some(reader.read::<24, u32>()?);
            }
            audio_object_type = AudioObjectType::read_from(reader)?;
            if audio_object_type == AudioObjectType::ERBSAC {
                extension_channel_configuration = Some(reader.read::<4, u8>()?);
            }
        }
        let specific_config = SpecificConfig::read_remaining_from(
            (
                audio_object_type,
                channel_configuration,
                sampling_frequency_index,
            ),
            reader,
        )?;
        let ep_config = match audio_object_type {
            AudioObjectType::ERAACLC
            | AudioObjectType::ERAACLTP
            | AudioObjectType::ERAACScalable
            | AudioObjectType::ERTwinVQ
            | AudioObjectType::ERBSAC
            | AudioObjectType::ERAACLD
            | AudioObjectType::ERCELP
            | AudioObjectType::ERHVXC
            | AudioObjectType::ERHILN
            | AudioObjectType::ERParametric
            | AudioObjectType::ERAACELD => Some(EpConfig::read_from(reader)?),
            _ => None,
        };
        let mut sync_extension_type = None;
        let mut sync_extension_audio_object_type = None;
        let mut sync_extension_audio_object_type5 = None;
        let mut sync_extension_audio_object_type22 = None;
        if extension_audio_object_type != AudioObjectType::SBR && reader.remaining_bits() >= 16 {
            sync_extension_type = Some(reader.read::<11, u16>()?);
            if sync_extension_type.unwrap() == 0x2b7 {
                sync_extension_audio_object_type = Some(AudioObjectType::read_from(reader)?);
                if sync_extension_audio_object_type.unwrap() == AudioObjectType::SBR {
                    sync_extension_audio_object_type5 =
                        Some(SyncExtensionAudioObjectType5::read_from(reader)?)
                }
                if sync_extension_audio_object_type.unwrap() == AudioObjectType::ERBSAC {
                    sync_extension_audio_object_type22 =
                        Some(SyncExtensionAudioObjectType22::read_from(reader)?)
                }
            }
        }
        Ok(Self {
            audio_object_type,
            sampling_frequency_index,
            sampling_frequency,
            channel_configuration,
            sbr_present_flag,
            ps_present_flag,
            extension_audio_object_type,
            extension_sampling_frequency_index,
            extension_sampling_frequency,
            extension_channel_configuration,
            specific_config,
            ep_config,
            sync_extension_type,
            sync_extension_audio_object_type,
            sync_extension_audio_object_type5,
            sync_extension_audio_object_type22,
        })
    }
}
impl<'a>
    BitwiseReadReaminingFrom<(AudioObjectType, u8, SamplingFrequencyIndex), BitstreamReader<'a>>
    for SpecificConfig
{
    type Error = AACCodecError;
    fn read_remaining_from(
        header: (AudioObjectType, u8, SamplingFrequencyIndex),
        reader: &mut BitstreamReader<'a>,
    ) -> Result<Self, Self::Error> {
        let (audio_object_type, channel_configuration, sampling_frequency_index) = header;
        match audio_object_type {
            AudioObjectType::AACMain
            | AudioObjectType::AACLC
            | AudioObjectType::AACSSR
            | AudioObjectType::AACLTP
            | AudioObjectType::AACScalable
            | AudioObjectType::TwinVQ
            | AudioObjectType::ERAACLC
            | AudioObjectType::ERAACLTP
            | AudioObjectType::ERAACScalable
            | AudioObjectType::ERTwinVQ
            | AudioObjectType::ERBSAC
            | AudioObjectType::ERAACLD => Ok(Self::Ga(GASpecificConfig::read_remaining_from(
                (
                    audio_object_type,
                    channel_configuration,
                    sampling_frequency_index,
                ),
                reader,
            )?)),
            AudioObjectType::CELP => Ok(Self::Celp(CelpSpecificConfig::read_from(reader)?)),
            AudioObjectType::HVXC => Ok(Self::Hvxc(HvxcSpecificConfig::read_from(reader)?)),
            AudioObjectType::TTSI => Ok(Self::TTS(TTSSpecificConfig::read_from(reader)?)),
            AudioObjectType::MainSynthetic
            | AudioObjectType::WavetableSynthesis
            | AudioObjectType::GeneralMIDI
            | AudioObjectType::AlgorithmicSynthesisAndAudioFX => Ok(Self::StructuredAudio(
                StructuredAudioSpecificConfig::read_from(reader)?,
            )),
            AudioObjectType::ERCELP => Ok(Self::ErrorResilientCelp(
                ErrorResilientCelpSpecificConfig::read_remaining_from(
                    sampling_frequency_index,
                    reader,
                )?,
            )),
            AudioObjectType::ERHVXC => Ok(Self::ErrorResilientHvxc(
                ErrorResilientHvxcSpecificConfig::read_from(reader)?,
            )),
            AudioObjectType::ERHILN | AudioObjectType::ERParametric => {
                unimplemented!("no spec on this element")
            }
            AudioObjectType::SSC => Ok(Self::SSC(SSCSpecificConfig::read_remaining_from(
                channel_configuration,
                reader,
            )?)),
            AudioObjectType::MPEGSurround => {
                unimplemented!("no spec on this element")
            }
            AudioObjectType::Layer1 | AudioObjectType::Layer2 | AudioObjectType::Layer3 => {
                Ok(Self::Mpeg1_2 {
                    extension: reader.read_bit()?,
                })
            }
            AudioObjectType::DST => Ok(Self::DST(DSTSpecificConfig::read_from(reader)?)),
            AudioObjectType::ALS => Ok(Self::ALS {
                fill_bits: reader.read::<5, u8>()?,
                config: ALSSpecificConfig::read_from(reader)?,
            }),
            AudioObjectType::SLS | AudioObjectType::SLSNonCore => {
                Ok(Self::SLS(SLSSpecificConfig::read_remaining_from(
                    (
                        audio_object_type,
                        channel_configuration,
                        sampling_frequency_index,
                    ),
                    reader,
                )?))
            }
            AudioObjectType::ERAACELD => Ok(Self::ELD(ELDSpecificConfig::read_remaining_from(
                channel_configuration,
                reader,
            )?)),
            AudioObjectType::SMRSimple | AudioObjectType::SMRMain => {
                unimplemented!("no spec on this element")
            }
            _ => Ok(Self::Reserved),
        }
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for EpConfig {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let ep_config = reader.read::<2, u8>()?;
        let error_protection_specific_config = if ep_config == 2 || ep_config == 3 {
            Some(ErrorProtectionSpecificConfig::read_from(reader)?)
        } else {
            None
        };
        let direct_mapping = if ep_config == 3 {
            Some(reader.read_bit()?)
        } else {
            None
        };
        if direct_mapping == Some(true) {
            panic!("spec says TBD here")
        }
        Ok(Self {
            ep_config,
            error_protection_specific_config,
            direct_mapping,
        })
    }
}

impl<'a> BitwiseReadFrom<BitstreamReader<'a>> for SyncExtensionAudioObjectType5 {
    type Error = AACCodecError;
    fn read_from(reader: &mut BitstreamReader) -> Result<Self, Self::Error> {
        let sbr_present_flag = reader.read_bit()?;
        if !sbr_present_flag {
            return Ok(Self {
                sbr_present_flag,
                extension_sampling_frequency_index: None,
                extension_sampling_frequency: None,
                sync_extension_type: None,
                ps_present_flag: None,
            });
        }
        let extension_sampling_frequency_index =
            SamplingFrequencyIndex::try_from(reader.read::<4, u8>()?)?;
        let extension_sampling_frequency =
            if extension_sampling_frequency_index == SamplingFrequencyIndex::Escape {
                Some(reader.read::<24, u32>()?)
            } else {
                None
            };
        let sync_extension_type = if reader.remaining_bits() >= 12 {
            Some(reader.read::<11, u16>()?)
        } else {
            None
        };
        let ps_present_flag = if let Some(sync_ext_type) = sync_extension_type
            && sync_ext_type == 0x548
        {
            Some(reader.read_bit()?)
        } else {
            None
        };
        Ok(Self {
            sbr_present_flag,
            extension_sampling_frequency_index: Some(extension_sampling_frequency_index),
            extension_sampling_frequency,
            sync_extension_type,
            ps_present_flag,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for SyncExtensionAudioObjectType22 {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let sbr_present_flag = reader.read_bit()?;
        let extension_sampling_frequency_index = if sbr_present_flag {
            Some(SamplingFrequencyIndex::try_from(reader.read::<4, u8>()?)?)
        } else {
            None
        };
        let extension_sampling_frequency = if let Some(index) = extension_sampling_frequency_index
            && index == SamplingFrequencyIndex::Escape
        {
            Some(reader.read::<24, u32>()?)
        } else {
            None
        };
        let extension_channel_configuration = reader.read::<4, u8>()?;
        Ok(Self {
            sbr_present_flag,
            extension_sampling_frequency_index,
            extension_sampling_frequency,
            extension_channel_configuration,
        })
    }
}
