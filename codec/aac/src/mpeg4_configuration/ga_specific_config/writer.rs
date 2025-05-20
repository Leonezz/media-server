use bitstream_io::BitWrite;
use utils::traits::writer::BitwiseWriteTo;

use crate::errors::AACCodecError;

use super::{GAExtension, GASpecificConfig};

impl<W: BitWrite> BitwiseWriteTo<W> for GASpecificConfig {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_bit(self.frame_length_flag)?;
        if let Some(delay) = self.core_coder_delay {
            writer.write_bit(true)?;
            writer.write::<14, u16>(delay)?;
        } else {
            writer.write_bit(false)?;
        }
        writer.write_bit(self.extension_flag)?;
        if let Some(config) = self.program_config_element.as_ref() {
            config.write_to(writer)?;
        }
        if let Some(nr) = self.layer_nr {
            writer.write::<3, u8>(nr)?;
        }
        if let Some(extension) = self.extension.as_ref() {
            extension.write_to(writer)?;
        }
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for GAExtension {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        if let Some(num_of_sub_frame) = self.num_of_sub_frame {
            writer.write::<5, u8>(num_of_sub_frame)?;
        }
        if let Some(layer_length) = self.layer_length {
            writer.write::<11, u16>(layer_length)?;
        }
        if let Some(flag) = self.aac_section_data_resilience_flag {
            writer.write_bit(flag)?;
        }
        if let Some(flag) = self.aac_scalefactor_data_resilience_flag {
            writer.write_bit(flag)?;
        }
        if let Some(flag) = self.aac_spectral_data_resilience_flag {
            writer.write_bit(flag)?;
        }
        writer.write_bit(self.extension_flag3)?;
        Ok(())
    }
}
