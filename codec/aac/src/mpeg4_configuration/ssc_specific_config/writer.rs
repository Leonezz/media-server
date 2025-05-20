use bitstream_io::BitWrite;
use utils::traits::writer::BitwiseWriteTo;

use crate::errors::AACCodecError;

use super::SSCSpecificConfig;

impl<W: BitWrite> BitwiseWriteTo<W> for SSCSpecificConfig {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write::<2, u8>(self.decoder_level)?;
        writer.write::<4, u8>(self.update_rate)?;
        writer.write::<2, u8>(self.synthesis_method)?;
        if let Some(ext) = self.mode_ext {
            writer.write::<2, u8>(ext)?;
        }
        if let Some(reserved) = self.reserved {
            writer.write::<2, u8>(reserved)?;
        }
        Ok(())
    }
}
