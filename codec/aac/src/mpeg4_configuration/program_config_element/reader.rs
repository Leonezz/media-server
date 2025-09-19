use bitstream_io::BitRead;
use codec_bitstream::reader::BitstreamReader;
use utils::traits::reader::BitwiseReadFrom;

use crate::{
    errors::AACCodecError,
    mpeg4_configuration::audio_specific_config::sampling_frequency_index::SamplingFrequencyIndex,
};

use super::{ChannelElement, MatrixMixdownIdx, ObjectType, ProgramConfigElement, ValidCCElement};

impl<R: BitRead> BitwiseReadFrom<R> for MatrixMixdownIdx {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let matrix_mixdown_idx = reader.read::<2, u8>()?;
        let pseudo_surround_enable = reader.read_bit()?;
        Ok(Self {
            matrix_mixdown_idx,
            pseudo_surround_enable,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for ChannelElement {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let is_cpe = reader.read_bit()?;
        let tag_select = reader.read::<4, u8>()?;
        Ok(Self { is_cpe, tag_select })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for ValidCCElement {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let is_ind_sw = reader.read_bit()?;
        let tag_select = reader.read::<4, u8>()?;
        Ok(Self {
            is_ind_sw,
            tag_select,
        })
    }
}

impl<'a> BitwiseReadFrom<BitstreamReader<'a>> for ProgramConfigElement {
    type Error = AACCodecError;
    fn read_from(reader: &mut BitstreamReader<'a>) -> Result<Self, Self::Error> {
        let element_instance_tag = reader.read::<4, u8>()?;
        let object_type: ObjectType = reader.read::<2, u8>()?.try_into()?;
        let sampling_frequency_index: SamplingFrequencyIndex =
            reader.read::<4, u8>()?.try_into()?;
        let num_front_channel_elements = reader.read::<4, u8>()?;
        let num_side_channel_elements = reader.read::<4, u8>()?;
        let num_back_channel_elements = reader.read::<4, u8>()?;
        let num_lfe_channel_elements = reader.read::<2, u8>()?;
        let num_assoc_data_elements = reader.read::<3, u8>()?;
        let num_valid_cc_elements = reader.read::<4, u8>()?;
        let mono_mixdown_present = reader.read_bit()?;
        let mono_mixdown_element_number = if mono_mixdown_present {
            Some(reader.read::<4, u8>()?)
        } else {
            None
        };
        let stereo_mixdown_present = reader.read_bit()?;
        let stereo_mixdown_element_number = if stereo_mixdown_present {
            Some(reader.read::<4, u8>()?)
        } else {
            None
        };
        let matrix_mixdown_idx_present = reader.read_bit()?;
        let matrix_mix_down_idx = if matrix_mixdown_idx_present {
            Some(MatrixMixdownIdx::read_from(reader)?)
        } else {
            None
        };
        let mut read_channel_elements = |len: u8| -> Result<Vec<ChannelElement>, Self::Error> {
            let mut elements = vec![];
            for _ in 0..len {
                elements.push(ChannelElement::read_from(reader)?);
            }
            Ok(elements)
        };
        let front_channel_elements = read_channel_elements(num_front_channel_elements)?;
        let side_channel_elements = read_channel_elements(num_side_channel_elements)?;
        let back_channel_elements = read_channel_elements(num_back_channel_elements)?;
        let read_u8_array = |len: u8,
                             bit_width: u32,
                             reader: &mut BitstreamReader<'a>|
         -> Result<Vec<u8>, Self::Error> {
            let mut select = vec![];
            for _ in 0..len {
                select.push(reader.read_var(bit_width)?);
            }
            Ok(select)
        };
        let lfe_element_tag_select = read_u8_array(num_lfe_channel_elements, 4, reader)?;
        let assoc_data_element_tag_select = read_u8_array(num_assoc_data_elements, 4, reader)?;
        let mut read_valid_cc_elements = |len: u8| -> Result<Vec<ValidCCElement>, Self::Error> {
            let mut elements = vec![];
            for _ in 0..len {
                elements.push(ValidCCElement::read_from(reader)?);
            }
            Ok(elements)
        };
        let valid_cc_elements = read_valid_cc_elements(num_valid_cc_elements)?;
        reader.byte_align();
        let comment_field_bytes = reader.read::<8, u8>()?;
        let comment_field_data = read_u8_array(comment_field_bytes, 8, reader)?;
        Ok(Self {
            element_instance_tag,
            object_type,
            sampling_frequency_index,
            num_front_channel_elements,
            num_side_channel_elements,
            num_back_channel_elements,
            num_lfe_channel_elements,
            num_assoc_data_elements,
            num_valid_cc_elements,
            mono_mixdown_present,
            mono_mixdown_element_number,
            stereo_mixdown_present,
            stereo_mixdown_element_number,
            matrix_mixdown_idx_present,
            matrix_mix_down_idx,
            front_channel_elements,
            side_channel_elements,
            back_channel_elements,
            lfe_element_tag_select,
            assoc_data_element_tag_select,
            valid_cc_elements,
            comment_field_bytes,
            comment_field_data,
        })
    }
}
