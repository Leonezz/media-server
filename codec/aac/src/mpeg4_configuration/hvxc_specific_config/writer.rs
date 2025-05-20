use bitstream_io::BitWrite;
use utils::traits::writer::BitwiseWriteTo;

use crate::errors::AACCodecError;

use super::{HVXCConfig, HvxcSpecificConfig};

impl<W: BitWrite> BitwiseWriteTo<W> for HvxcSpecificConfig {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        if let Some(config) = self.config.as_ref() {
            writer.write_bit(true)?;
            config.write_to(writer)?;
        } else {
            writer.write_bit(false)?;
        }
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for HVXCConfig {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_bit(self.hvxc_var_mode.into())?;
        writer.write::<2, u8>(self.hvxc_rate_mode.into())?;
        writer.write_bit(self.extension_flag)?;
        Ok(())
    }
}
