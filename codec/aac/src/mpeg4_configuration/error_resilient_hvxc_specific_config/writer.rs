use bitstream_io::BitWrite;
use utils::traits::writer::BitwiseWriteTo;

use crate::errors::AACCodecError;

use super::{ErHvxcConfig, ErrorResilientHvxcSpecificConfig};

impl<W: BitWrite> BitwiseWriteTo<W> for ErrorResilientHvxcSpecificConfig {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        if let Some(config) = self.config {
            writer.write_bit(true)?;
            config.write_to(writer)?;
        } else {
            writer.write_bit(false)?;
        }
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for ErHvxcConfig {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_bit(self.hvxc_var_mode.into())?;
        writer.write::<2, u8>(self.hvxc_rate_mode.into())?;
        if let Some(flag) = self.var_scalable_flag {
            writer.write_bit(true)?;
            writer.write_bit(flag)?;
        } else {
            writer.write_bit(false)?;
        }
        Ok(())
    }
}
