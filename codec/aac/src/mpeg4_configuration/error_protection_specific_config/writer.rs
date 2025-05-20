use bitstream_io::BitWrite;
use num::ToPrimitive;
use utils::traits::writer::BitwiseWriteTo;

use crate::errors::AACCodecError;

use super::{ErrorProtectionSpecificConfig, PredefinedSet, PredefinedSetClass};

impl<W: BitWrite> BitwiseWriteTo<W> for ErrorProtectionSpecificConfig {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write::<8, u8>(self.predefined_sets.len().to_u8().unwrap())?;
        writer.write::<2, u8>(self.interleave_type)?;
        writer.write::<3, u8>(self.bit_stuffing)?;
        writer.write::<3, u8>(self.number_of_concatenated_frame)?;
        self.predefined_sets
            .iter()
            .try_for_each(|item| item.write_to(writer))?;
        writer.write_bit(self.header_protection)?;
        if let Some(header_rate) = self.header_rate {
            writer.write::<5, u8>(header_rate)?;
        }
        if let Some(header_crclen) = self.header_crclen {
            writer.write::<5, u8>(header_crclen)?;
        }
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for PredefinedSet {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write::<6, u8>(self.class.len().to_u8().unwrap())?;
        self.class
            .iter()
            .try_for_each(|item| item.write_to(writer))?;
        if let Some(order) = self.class_output_order.as_ref() {
            writer.write_bit(true)?;
            order
                .iter()
                .try_for_each(|item| writer.write::<6, u8>(*item))?;
        } else {
            writer.write_bit(false)?;
        }
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for PredefinedSetClass {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_bit(self.length_escape)?;
        writer.write_bit(self.rate_escape)?;
        writer.write_bit(self.crclen_escape)?;
        if let Some(flag) = self.concatenate_flag {
            writer.write_bit(flag)?;
        }
        writer.write::<2, u8>(self.fec_type)?;
        if let Some(flag) = self.termination_switch {
            writer.write_bit(flag)?;
        }
        if let Some(interleave) = self.interleave_switch {
            writer.write::<2, u8>(interleave)?;
        }
        writer.write_bit(self.class_optional)?;
        if let Some(num_bits_for_length) = self.number_of_bits_for_length {
            writer.write::<4, u8>(num_bits_for_length)?;
        }
        if let Some(class_length) = self.class_length {
            writer.write::<16, u16>(class_length)?;
        }
        if let Some(class_rate7) = self.class_rate_7bits {
            writer.write::<7, u8>(class_rate7)?;
        }
        if let Some(class_rate5) = self.class_rate_5bits {
            writer.write::<5, u8>(class_rate5)?;
        }
        if let Some(class_crclen) = self.class_crclen {
            writer.write::<5, u8>(class_crclen)?;
        }
        Ok(())
    }
}
