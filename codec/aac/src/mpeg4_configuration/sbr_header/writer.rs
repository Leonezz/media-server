use bitstream_io::BitWrite;
use utils::traits::writer::BitwiseWriteTo;

use crate::errors::AACCodecError;

use super::{BsHeaderExtra1, BsHeaderExtra2, SbrHeader};

impl<W: BitWrite> BitwiseWriteTo<W> for BsHeaderExtra1 {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write::<2, u8>(self.bs_freq_scale)?;
        writer.write_bit(self.bs_alter_scale)?;
        writer.write::<2, u8>(self.bs_noise_bands)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for BsHeaderExtra2 {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write::<2, u8>(self.bs_limiter_bands)?;
        writer.write::<2, u8>(self.bs_limiter_gains)?;
        writer.write_bit(self.bs_interpol_freq)?;
        writer.write_bit(self.bs_smoothing_mode)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for SbrHeader {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_bit(self.bs_amp_res)?;
        writer.write::<4, u8>(self.bs_start_freq)?;
        writer.write::<4, u8>(self.bs_stop_freq)?;
        writer.write::<3, u8>(self.bs_xover_band)?;
        writer.write::<2, u8>(self.bs_reserved)?;
        writer.write_bit(self.bs_header_extra_1)?;
        writer.write_bit(self.bs_header_extra_2)?;
        if let Some(extra1) = self.extra1 {
            extra1.write_to(writer)?;
        }
        if let Some(extra2) = self.extra2 {
            extra2.write_to(writer)?;
        }
        Ok(())
    }
}
