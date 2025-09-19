use bitstream_io::BitRead;
use utils::traits::reader::BitwiseReadFrom;

use crate::errors::AACCodecError;

use super::{BsHeaderExtra1, BsHeaderExtra2, SbrHeader};

impl<R: BitRead> BitwiseReadFrom<R> for BsHeaderExtra1 {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let bs_freq_scale = reader.read::<2, u8>()?;
        let bs_alter_scale = reader.read_bit()?;
        let bs_noise_bands = reader.read::<2, u8>()?;
        Ok(Self {
            bs_freq_scale,
            bs_alter_scale,
            bs_noise_bands,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for BsHeaderExtra2 {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let bs_limiter_bands = reader.read::<2, u8>()?;
        let bs_limiter_gains = reader.read::<2, u8>()?;
        let bs_interpol_freq = reader.read_bit()?;
        let bs_smoothing_mode = reader.read_bit()?;
        Ok(Self {
            bs_limiter_bands,
            bs_limiter_gains,
            bs_interpol_freq,
            bs_smoothing_mode,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for SbrHeader {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let bs_amp_res = reader.read_bit()?;
        let bs_start_freq = reader.read::<4, u8>()?;
        let bs_stop_freq = reader.read::<4, u8>()?;
        let bs_xover_band = reader.read::<3, u8>()?;
        let bs_reserved = reader.read::<2, u8>()?;
        let bs_header_extra_1 = reader.read_bit()?;
        let bs_header_extra_2 = reader.read_bit()?;
        let extra1 = if bs_header_extra_1 {
            Some(BsHeaderExtra1::read_from(reader)?)
        } else {
            None
        };
        let extra2 = if bs_header_extra_2 {
            Some(BsHeaderExtra2::read_from(reader)?)
        } else {
            None
        };
        Ok(Self {
            bs_amp_res,
            bs_start_freq,
            bs_stop_freq,
            bs_xover_band,
            bs_reserved,
            bs_header_extra_1,
            bs_header_extra_2,
            extra1,
            extra2,
        })
    }
}
