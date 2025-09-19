use bitstream_io::BitRead;
use codec_bitstream::reader::BitstreamReader;
use utils::traits::reader::{BitwiseReadFrom, BitwiseReadReaminingFrom};

use crate::{
    errors::AACCodecError,
    mpeg4_configuration::{
        audio_specific_config::{
            audio_object_type::AudioObjectType, sampling_frequency_index::SamplingFrequencyIndex,
        },
        program_config_element::ProgramConfigElement,
    },
};

use super::{GAExtension, GASpecificConfig};

impl<R: BitRead> BitwiseReadReaminingFrom<AudioObjectType, R> for GAExtension {
    type Error = AACCodecError;
    fn read_remaining_from(
        audio_object_type: AudioObjectType,
        reader: &mut R,
    ) -> Result<Self, Self::Error> {
        let (num_of_sub_frame, layer_length) = if audio_object_type == AudioObjectType::ERBSAC {
            (
                Some(reader.read::<5, u8>()?),
                Some(reader.read::<11, u16>()?),
            )
        } else {
            (None, None)
        };
        let (
            aac_section_data_resilience_flag,
            aac_scalefactor_data_resilience_flag,
            aac_spectral_data_resilience_flag,
        ) = match audio_object_type {
            AudioObjectType::ERAACLC
            | AudioObjectType::ERAACLTP
            | AudioObjectType::ERAACScalable
            | AudioObjectType::ERAACLD => (
                Some(reader.read_bit()?),
                Some(reader.read_bit()?),
                Some(reader.read_bit()?),
            ),
            _ => (None, None, None),
        };
        let extension_flag3 = reader.read_bit()?;
        if extension_flag3 {
            panic!("the spec says TBD here");
        }
        Ok(Self {
            num_of_sub_frame,
            layer_length,
            aac_section_data_resilience_flag,
            aac_scalefactor_data_resilience_flag,
            aac_spectral_data_resilience_flag,
            extension_flag3,
        })
    }
}

impl<'a>
    BitwiseReadReaminingFrom<(AudioObjectType, u8, SamplingFrequencyIndex), BitstreamReader<'a>>
    for GASpecificConfig
{
    type Error = AACCodecError;
    fn read_remaining_from(
        header: (AudioObjectType, u8, SamplingFrequencyIndex),
        reader: &mut BitstreamReader<'a>,
    ) -> Result<Self, Self::Error> {
        let (audio_object_type, channel_configuration, _sampling_frequency_index) = header;
        let frame_length_flag = reader.read_bit()?;
        let depends_on_core_coder = reader.read_bit()?;
        let core_coder_delay = if depends_on_core_coder {
            Some(reader.read::<14, u16>()?)
        } else {
            None
        };
        let extension_flag = reader.read_bit()?;
        let program_config_element = if channel_configuration == 0 {
            Some(ProgramConfigElement::read_from(reader)?)
        } else {
            None
        };
        let layer_nr = if audio_object_type == AudioObjectType::AACScalable
            || audio_object_type == AudioObjectType::ERAACScalable
        {
            Some(reader.read::<3, u8>()?)
        } else {
            None
        };
        let extension = if extension_flag {
            Some(GAExtension::read_remaining_from(audio_object_type, reader)?)
        } else {
            None
        };
        Ok(Self {
            frame_length_flag,
            depends_on_core_coder,
            core_coder_delay,
            extension_flag,
            program_config_element,
            layer_nr,
            extension,
        })
    }
}
