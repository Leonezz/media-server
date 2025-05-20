use std::ops::Rem;

use bitstream_io::BitWrite;
use num::ToPrimitive;
use utils::traits::writer::BitwiseWriteTo;

use crate::errors::AACCodecError;

use super::{ALSSpecificConfig, AuxData};

impl<W: BitWrite> BitwiseWriteTo<W> for AuxData {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write::<32, u32>(self.aux_size)?;
        self.aux_data
            .iter()
            .try_for_each(|item| writer.write::<8, u8>(*item))?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for ALSSpecificConfig {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let mut len_write = 0;
        writer.write::<32, u32>(self.als_id)?;
        len_write += 32;
        writer.write::<32, u32>(self.samp_freq)?;
        len_write += 32;
        writer.write::<32, u32>(self.samples)?;
        len_write += 32;
        writer.write::<16, u16>(self.channels)?;
        len_write += 16;
        writer.write::<3, u8>(self.file_type)?;
        len_write += 3;
        writer.write::<3, u8>(self.resolution)?;
        len_write += 3;
        writer.write_bit(self.floating)?;
        len_write += 1;
        writer.write_bit(self.msb_first)?;
        len_write += 1;
        writer.write::<16, u16>(self.frame_length)?;
        len_write += 16;
        writer.write::<8, u8>(self.random_access)?;
        len_write += 8;
        writer.write::<2, u8>(self.ra_flag)?;
        len_write += 2;
        writer.write_bit(self.adapt_order)?;
        len_write += 1;
        writer.write::<2, u8>(self.coef_table)?;
        len_write += 2;
        writer.write_bit(self.long_term_prediction)?;
        len_write += 1;
        writer.write::<10, u16>(self.max_order)?;
        len_write += 10;
        writer.write::<2, u8>(self.block_switching)?;
        len_write += 2;
        writer.write_bit(self.bgmc_mode)?;
        len_write += 1;
        writer.write_bit(self.sb_part)?;
        len_write += 1;
        writer.write_bit(self.joint_stereo)?;
        len_write += 1;
        writer.write_bit(self.mc_coding)?;
        len_write += 1;
        writer.write_bit(self.chan_config)?;
        len_write += 1;
        writer.write_bit(self.chan_sort)?;
        len_write += 1;
        writer.write_bit(self.crc_enabled)?;
        len_write += 1;
        writer.write_bit(self.rlslms)?;
        len_write += 1;
        writer.write::<5, u8>(self.reserved)?;
        len_write += 5;
        writer.write_bit(self.aux_data_enabled)?;
        len_write += 1;
        if let Some(info) = self.chan_config_info {
            writer.write::<16, u16>(info)?;
            len_write += 16;
        }
        if let Some(chan_pos) = self.chan_pos.as_ref() {
            let chan_pos_bits = self
                .channels
                .checked_add(1)
                .and_then(|v| v.to_f64())
                .and_then(|v| v.log2().ceil().to_u32())
                .unwrap();

            chan_pos
                .iter()
                .try_for_each(|item| writer.write_var(chan_pos_bits, *item))?;
            len_write += chan_pos.len() * chan_pos_bits.to_usize().unwrap();
        }
        let to_align = 8_usize
            .checked_sub(len_write.rem(8))
            .and_then(|v| v.to_u32())
            .unwrap();
        if to_align > 0 {
            writer.write_var(to_align, 0)?;
        }
        writer.write::<32, u32>(self.header_size)?;
        writer.write::<32, u32>(self.trailer_size)?;
        self.orig_header
            .iter()
            .try_for_each(|item| writer.write::<8, u8>(*item))?;
        self.orig_trailer
            .iter()
            .try_for_each(|item| writer.write::<8, u8>(*item))?;
        if let Some(crc) = self.crc {
            writer.write::<32, u32>(crc)?;
        }
        if let Some(ra_unit_size) = self.ra_unit_size.as_ref() {
            ra_unit_size
                .iter()
                .try_for_each(|item| writer.write::<32, u32>(*item))?;
        }
        if let Some(aux_data) = self.aux_data.as_ref() {
            aux_data.write_to(writer)?;
        }
        Ok(())
    }
}
