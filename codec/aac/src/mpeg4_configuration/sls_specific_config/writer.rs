use bitstream_io::BitWrite;
use utils::traits::writer::BitwiseWriteTo;

use crate::errors::AACCodecError;

use super::SLSSpecificConfig;

impl<W: BitWrite> BitwiseWriteTo<W> for SLSSpecificConfig {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write::<3, u8>(self.pcm_word_length)?;
        writer.write_bit(self.aac_core_present)?;
        writer.write_bit(self.lle_main_stream)?;
        writer.write_bit(self.reserved_bit)?;
        writer.write::<3, u8>(self.frame_length)?;
        if let Some(config) = self.program_config_element.as_ref() {
            config.write_to(writer)?;
        }
        Ok(())
    }
}
