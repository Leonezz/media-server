use byteorder::{BigEndian, ReadBytesExt};
use utils::traits::reader::ReadFrom;

use std::io;

use crate::errors::FLVError;

use super::{EncryptionFilterParams, EncryptionTagHeader, SelectiveEncryptionFilterParams};

impl<R: io::Read> ReadFrom<R> for EncryptionTagHeader {
    type Error = FLVError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let num_filters = reader.read_u8()?;
        let name = amf_formats::amf0::Value::read_from(reader)?;
        let filter_name = match name {
            amf_formats::amf0::Value::String(str) => str,
            _ => {
                return Err(FLVError::UnexpectedValue(format!(
                    "expect string for encryption tag header filter name, got {:?} instead",
                    name
                )));
            }
        };

        let length = reader.read_u24::<BigEndian>()?;
        Ok(EncryptionTagHeader {
            num_filters,
            filter_name,
            length,
        })
    }
}

impl<R: io::Read> ReadFrom<R> for EncryptionFilterParams {
    type Error = FLVError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let mut iv = [0; 16];
        reader.read_exact(&mut iv)?;
        Ok(Self { iv })
    }
}

impl<R: io::Read> ReadFrom<R> for SelectiveEncryptionFilterParams {
    type Error = FLVError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let byte = reader.read_u8()?;
        let au = ((byte >> 7) & 0b1) != 0;
        if au {
            let mut iv: [u8; 16] = [0; 16];
            reader.read_exact(&mut iv)?;
            return Ok(Self { iv: Some(iv) });
        }
        Ok(SelectiveEncryptionFilterParams { iv: None })
    }
}
