use bitstream_io::BitWrite;
use num::ToPrimitive;
use utils::traits::writer::BitwiseWriteTo;

use crate::errors::AACCodecError;

use super::{ChannelElement, MatrixMixdownIdx, ProgramConfigElement, ValidCCElement};

impl<W: BitWrite> BitwiseWriteTo<W> for MatrixMixdownIdx {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write::<2, u8>(self.matrix_mixdown_idx)?;
        writer.write_bit(self.pseudo_surround_enable)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for ChannelElement {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_bit(self.is_cpe)?;
        writer.write::<4, u8>(self.tag_select)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for ValidCCElement {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_bit(self.is_ind_sw)?;
        writer.write::<4, u8>(self.tag_select)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for ProgramConfigElement {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write::<4, u8>(self.element_instance_tag)?;
        writer.write::<2, u8>(self.object_type.into())?;
        writer.write::<4, u8>(self.sampling_frequency_index.into())?;
        writer.write::<4, u8>(self.front_channel_elements.len().to_u8().unwrap())?;
        writer.write::<4, u8>(self.side_channel_elements.len().to_u8().unwrap())?;
        writer.write::<4, u8>(self.back_channel_elements.len().to_u8().unwrap())?;
        writer.write::<2, u8>(self.lfe_element_tag_select.len().to_u8().unwrap())?;
        writer.write::<3, u8>(self.assoc_data_element_tag_select.len().to_u8().unwrap())?;
        writer.write::<4, u8>(self.valid_cc_elements.len().to_u8().unwrap())?;
        if let Some(mono) = self.mono_mixdown_element_number {
            writer.write_bit(true)?;
            writer.write::<4, u8>(mono)?;
        } else {
            writer.write_bit(false)?;
        }
        if let Some(stereo) = self.stereo_mixdown_element_number {
            writer.write_bit(true)?;
            writer.write::<4, u8>(stereo)?;
        } else {
            writer.write_bit(false)?;
        }
        if let Some(matrix) = self.matrix_mix_down_idx {
            writer.write_bit(true)?;
            matrix.write_to(writer)?;
        } else {
            writer.write_bit(false)?;
        }
        self.front_channel_elements
            .iter()
            .try_for_each(|item| item.write_to(writer))?;
        self.side_channel_elements
            .iter()
            .try_for_each(|item| item.write_to(writer))?;
        self.back_channel_elements
            .iter()
            .try_for_each(|item| item.write_to(writer))?;
        self.lfe_element_tag_select
            .iter()
            .try_for_each(|item| writer.write::<4, u8>(*item))?;
        self.assoc_data_element_tag_select
            .iter()
            .try_for_each(|item| writer.write::<4, u8>(*item))?;
        self.valid_cc_elements
            .iter()
            .try_for_each(|item| item.write_to(writer))?;
        writer.byte_align()?;
        writer.write::<8, u8>(self.comment_field_data.len().to_u8().unwrap())?;
        writer.write_bytes(&self.comment_field_data)?;
        Ok(())
    }
}
