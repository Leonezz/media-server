use byteorder::{BigEndian, WriteBytesExt};
use num::ToPrimitive;
use std::io;
use utils::traits::writer::WriteTo;

use crate::errors::H264CodecError;

use super::{AvcDecoderConfigurationRecord, SpsExtRelated};

impl<W: io::Write> WriteTo<W> for SpsExtRelated {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u8(Into::<u8>::into(self.chroma_format_idc) | 0b1111_1100)?;
        writer.write_u8(self.bit_depth_luma_minus8 | 0b1111_1000)?;
        writer.write_u8(self.bit_depth_chroma_minus8 | 0b1111_1000)?;
        writer.write_u8(self.sequence_parameter_set_ext.len().to_u8().unwrap())?; // num_of_sequence_parameter_ext
        self.sequence_parameter_set_ext
            .iter()
            .try_for_each(|item| {
                writer.write_u16::<BigEndian>(item.sequence_parameter_set_length)?;
                item.nalu.write_to(writer)?;
                Ok::<(), Self::Error>(())
            })?;
        Ok(())
    }
}

impl<W: io::Write> WriteTo<W> for AvcDecoderConfigurationRecord {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u8(self.configuration_version)?;
        writer.write_u8(self.avc_profile_indication)?;
        writer.write_u8(self.profile_compatibility)?;
        writer.write_u8(self.avc_level_indication)?;
        writer.write_u8(
            (self.length_size_minus_one & 0b11) | ((self.reserved_6_bits_1 & 0b111111) << 2),
        )?;
        writer.write_u8(
            (self.num_of_sequence_parameter_sets & 0b11111)
                | ((self.reserved_3_bits_1 & 0b111) << 5),
        )?;
        self.sequence_parameter_sets.iter().try_for_each(|item| {
            writer.write_u16::<BigEndian>(item.sequence_parameter_set_length)?;
            item.nalu.write_to(writer)?;
            Ok::<(), Self::Error>(())
        })?;
        writer.write_u8(self.num_of_picture_parameter_sets)?;
        self.picture_parameter_sets.iter().try_for_each(|item| {
            writer.write_u16::<BigEndian>(item.sequence_parameter_set_length)?;
            item.nalu.write_to(writer)?;
            Ok::<(), Self::Error>(())
        })?;
        if let Some(sps_ext_related) = &self.sps_ext_related {
            sps_ext_related.write_to(writer)?;
        }
        Ok(())
    }
}
