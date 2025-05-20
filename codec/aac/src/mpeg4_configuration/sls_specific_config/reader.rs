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

use super::SLSSpecificConfig;

impl<'a>
    BitwiseReadReaminingFrom<(AudioObjectType, u8, SamplingFrequencyIndex), BitstreamReader<'a>>
    for SLSSpecificConfig
{
    type Error = AACCodecError;
    fn read_remaining_from(
        header: (AudioObjectType, u8, SamplingFrequencyIndex),
        reader: &mut BitstreamReader<'a>,
    ) -> Result<Self, Self::Error> {
        let (_audio_object_type, channel_configuration, _sampling_frequency_index) = header;
        let pcm_word_length = reader.read::<3, u8>()?;
        let aac_core_present = reader.read_bit()?;
        let lle_main_stream = reader.read_bit()?;
        let reserved_bit = reader.read_bit()?;
        let frame_length = reader.read::<3, u8>()?;
        let program_config_element = if channel_configuration != 0 {
            Some(ProgramConfigElement::read_from(reader)?)
        } else {
            None
        };
        Ok(SLSSpecificConfig {
            pcm_word_length,
            aac_core_present,
            lle_main_stream,
            reserved_bit,
            frame_length,
            program_config_element,
        })
    }
}
