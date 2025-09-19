use bitstream_io::BitWrite;
use num::ToPrimitive;
use utils::traits::writer::BitwiseWriteTo;

use crate::errors::AACCodecError;

use super::{ELDEXT_TERM, ELDSpecificConfig, EldExtData, LdSbr, LdSbrHeader};

impl<W: BitWrite> BitwiseWriteTo<W> for ELDSpecificConfig {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_bit(self.frame_length_flag)?;
        writer.write_bit(self.aac_section_data_resilience_flag)?;
        writer.write_bit(self.aac_scalefactor_data_resilience_flag)?;
        writer.write_bit(self.aac_spectral_data_resilience_flag)?;
        if let Some(ld_sbr) = self.ld_sbr.as_ref() {
            writer.write_bit(true)?; // ldSbrPresentFlag
            ld_sbr.write_to(writer)?;
        } else {
            writer.write_bit(false)?;
        }
        self.eld_ext_data
            .iter()
            .try_for_each(|item| item.write_to(writer))?;

        writer.write::<4, u8>(ELDEXT_TERM)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for EldExtData {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write::<4, u8>(self.eld_ext_type)?;
        let len = self.other_byte.len();
        if len < 15 {
            writer.write::<4, u8>(len.to_u8().unwrap())?;
        } else if len < 255 + 15 {
            writer.write::<4, u8>(15)?;
            writer.write::<8, u8>((len - 15).to_u8().unwrap())?;
        } else {
            writer.write::<4, u8>(15)?;
            writer.write::<8, u8>(255)?;
            writer.write::<16, u16>((len - 255 - 15).to_u16().unwrap())?;
        }
        writer.write_bytes(&self.other_byte)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for LdSbr {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_bit(self.ld_sbr_sampling_rate)?;
        writer.write_bit(self.ld_sbr_crc_flag)?;
        self.ld_sbr_header.write_to(writer)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for LdSbrHeader {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        self.sbr_headers
            .iter()
            .try_for_each(|item| item.write_to(writer))
    }
}
