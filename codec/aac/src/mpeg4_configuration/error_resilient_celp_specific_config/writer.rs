use bitstream_io::BitWrite;
use utils::traits::writer::BitwiseWriteTo;

use crate::errors::AACCodecError;

use super::{ErScCelpHeader, ErrorResilientCelpSpecificConfig};

impl<W: BitWrite> BitwiseWriteTo<W> for ErrorResilientCelpSpecificConfig {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        if let Some(celp_header) = self.er_sc_celp_header.as_ref() {
            writer.write_bit(true)?;
            celp_header.write_to(writer)?;
        } else {
            writer.write_bit(false)?;
            if let Some(senh_header) = self.celp_bw_senh_header {
                writer.write_bit(true)?;
                senh_header.write_to(writer)?;
            } else {
                writer.write_bit(false)?;
                writer.write::<2, u8>(self.celp_brs_id.unwrap())?;
            }
        }
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for ErScCelpHeader {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_bit(self.excitation_mode.into())?;
        writer.write_bit(self.sample_rate_mode.into())?;
        writer.write_bit(self.fine_rate_control.into())?;
        writer.write_bit(self.silence_compression)?;
        if let Some(rpe) = self.rpe_configuration {
            writer.write::<3, u8>(rpe)?;
        }
        if let Some(mpe) = self.excitation_mode_mpe {
            mpe.write_to(writer)?;
        }
        Ok(())
    }
}
