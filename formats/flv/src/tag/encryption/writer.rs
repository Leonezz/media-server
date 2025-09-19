use byteorder::{BigEndian, WriteBytesExt};
use utils::traits::writer::WriteTo;

use std::io;

use crate::errors::FLVError;

use super::{EncryptionFilterParams, EncryptionTagHeader, SelectiveEncryptionFilterParams};

impl<W: io::Write> WriteTo<W> for EncryptionTagHeader {
    type Error = FLVError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u8(self.num_filters)?;
        amf_formats::amf0::Value::write_string(writer, &self.filter_name)?;
        writer.write_u24::<BigEndian>(self.length)?;
        Ok(())
    }
}

impl<W: io::Write> WriteTo<W> for EncryptionFilterParams {
    type Error = FLVError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_all(&self.iv)?;
        Ok(())
    }
}

impl<W: io::Write> WriteTo<W> for SelectiveEncryptionFilterParams {
    type Error = FLVError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        if let Some(iv) = self.iv {
            writer.write_u8(0b1000_0000)?;
            writer.write_all(&iv)?;
        } else {
            writer.write_u8(0)?;
        }
        Ok(())
    }
}
