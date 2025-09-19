use bitstream_io::BitWrite;
use utils::traits::writer::BitwiseWriteTo;

use crate::errors::AACCodecError;

use super::DSTSpecificConfig;

impl<W: BitWrite> BitwiseWriteTo<W> for DSTSpecificConfig {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_bit(self.dsddst_coded)?;
        writer.write::<14, u16>(self.n_channels)?;
        writer.write_bit(self.reserved)?;
        Ok(())
    }
}
