use std::ops::{Add, Rem};

use bitstream_io::BitRead;
use num::ToPrimitive;
use utils::traits::reader::BitwiseReadFrom;

use crate::errors::AACCodecError;

use super::{ALSSpecificConfig, AuxData};

impl<R: BitRead> BitwiseReadFrom<R> for AuxData {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let aux_size = reader.read::<32, u32>()?;
        let aux_data = {
            let mut data = vec![];
            for _ in 0..aux_size {
                data.push(reader.read::<8, u8>()?);
            }
            data
        };
        Ok(Self { aux_size, aux_data })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for ALSSpecificConfig {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let mut bits_read: usize = 0;
        let als_id = reader.read::<32, u32>()?;
        bits_read += 32;
        let samp_freq = reader.read::<32, u32>()?;
        bits_read += 32;
        let samples = reader.read::<32, u32>()?;
        bits_read += 32;
        let channels = reader.read::<16, u16>()?;
        bits_read += 16;
        let file_type = reader.read::<3, u8>()?;
        bits_read += 3;
        let resolution = reader.read::<3, u8>()?;
        bits_read += 3;
        let floating = reader.read_bit()?;
        bits_read += 1;
        let msb_first = reader.read_bit()?;
        bits_read += 1;
        let frame_length = reader.read::<16, u16>()?;
        bits_read += 16;
        let random_access = reader.read::<8, u8>()?;
        bits_read += 8;
        let ra_flag = reader.read::<2, u8>()?;
        bits_read += 2;
        let adapt_order = reader.read_bit()?;
        bits_read += 1;
        let coef_table = reader.read::<2, u8>()?;
        bits_read += 2;
        let long_term_prediction = reader.read_bit()?;
        bits_read += 1;
        let max_order = reader.read::<10, u16>()?;
        bits_read += 10;
        let block_switching = reader.read::<2, u8>()?;
        bits_read += 2;
        let bgmc_mode = reader.read_bit()?;
        bits_read += 1;
        let sb_part = reader.read_bit()?;
        bits_read += 1;
        let joint_stereo = reader.read_bit()?;
        bits_read += 1;
        let mc_coding = reader.read_bit()?;
        bits_read += 1;
        let chan_config = reader.read_bit()?;
        bits_read += 1;
        let chan_sort = reader.read_bit()?;
        bits_read += 1;
        let crc_enabled = reader.read_bit()?;
        bits_read += 1;
        let rlslms = reader.read_bit()?;
        bits_read += 1;
        let reserved = reader.read::<5, u8>()?;
        bits_read += 5;
        let aux_data_enabled = reader.read_bit()?;
        bits_read += 1;
        let chan_config_info = if chan_config {
            let res = Some(reader.read::<16, u16>()?);
            bits_read += 16;
            res
        } else {
            None
        };
        let chan_pos = if chan_sort {
            let bits_cnt = channels
                .to_f64()
                .unwrap()
                .add(1.0)
                .log2()
                .ceil()
                .to_u32()
                .unwrap();
            assert!(bits_cnt <= 16);
            let mut pos = vec![];
            for _ in 0..=channels {
                pos.push(reader.read_var(bits_cnt)?);
            }
            bits_read += pos.len() * bits_cnt.to_usize().unwrap();
            Some(pos)
        } else {
            None
        };
        let byte_align = reader.read_var(
            8_usize
                .checked_sub(bits_read.rem(8))
                .unwrap()
                .to_u32()
                .unwrap(),
        )?;
        let header_size = reader.read::<32, u32>()?;
        let trailer_size = reader.read::<32, u32>()?;
        let orig_header = {
            let mut header = vec![];
            for _ in 0..header_size {
                header.push(reader.read::<8, u8>()?);
            }
            header
        };
        let orig_trailer = {
            let mut trailer = vec![];
            for _ in 0..trailer_size {
                trailer.push(reader.read::<8, u8>()?);
            }
            trailer
        };
        let crc = if crc_enabled {
            Some(reader.read::<32, u32>()?)
        } else {
            None
        };
        let ra_unit_size = if ra_flag == 2 && random_access > 0 {
            let cnt = (samples - 1) / (frame_length.to_u32().unwrap() + 1) + 1;
            let mut unit_size = vec![];
            for _ in 0..cnt {
                unit_size.push(reader.read::<32, u32>()?);
            }
            Some(unit_size)
        } else {
            None
        };
        let aux_data = if aux_data_enabled {
            Some(AuxData::read_from(reader)?)
        } else {
            None
        };
        Ok(Self {
            als_id,
            samp_freq,
            samples,
            channels,
            file_type,
            resolution,
            floating,
            msb_first,
            frame_length,
            random_access,
            ra_flag,
            adapt_order,
            coef_table,
            long_term_prediction,
            max_order,
            block_switching,
            bgmc_mode,
            sb_part,
            joint_stereo,
            mc_coding,
            chan_config,
            chan_sort,
            crc_enabled,
            rlslms,
            reserved,
            aux_data_enabled,
            chan_config_info,
            chan_pos,
            byte_align,
            header_size,
            trailer_size,
            orig_header,
            orig_trailer,
            crc,
            ra_unit_size,
            aux_data,
        })
    }
}
