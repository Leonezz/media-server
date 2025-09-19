use bitstream_io::BitWrite;
use utils::traits::writer::BitwiseWriteTo;

use crate::errors::AACCodecError;

use super::CelpSpecificConfig;

impl<W: BitWrite> BitwiseWriteTo<W> for CelpSpecificConfig {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        if let Some(celp_header) = self.celp_header.as_ref() {
            writer.write_bit(true)?; // isBaseLayer
            celp_header.write_to(writer)?;
        } else {
            writer.write_bit(false)?; // isBaseLayer
            if let Some(celp_bwsenh_header) = self.celp_bwsenh_header {
                writer.write_bit(true)?; // isBWSLayer
                celp_bwsenh_header.write_to(writer)?;
            } else {
                writer.write_bit(false)?;
                writer.write::<2, u8>(self.celp_brs_id.unwrap())?;
            }
        }
        Ok(())
    }
}
