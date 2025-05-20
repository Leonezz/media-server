use bitstream_io::BitWrite;
use utils::traits::writer::BitwiseWriteTo;

use crate::errors::AACCodecError;

use super::{CelpBWSenhHeader, CelpHeader, MPEExciationMode};

impl<W: BitWrite> BitwiseWriteTo<W> for MPEExciationMode {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write::<5, u8>(self.mpe_configuration)?;
        writer.write::<2, u8>(self.num_enh_layers)?;
        writer.write_bit(self.bandwidth_scalability_mode)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for CelpHeader {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_bit(self.excitation_mode.into())?;
        writer.write_bit(self.sample_rate_mode.into())?;
        writer.write_bit(self.fine_rate_control.into())?;
        if let Some(rpe) = self.rpe_configuration {
            writer.write::<3, u8>(rpe)?;
        }
        if let Some(mpe) = self.mpe_exciation_mode {
            mpe.write_to(writer)?;
        }
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for CelpBWSenhHeader {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write::<2, u8>(self.bws_configuration)?;
        Ok(())
    }
}
