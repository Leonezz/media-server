use byteorder::{BigEndian, ReadBytesExt};

use std::io;

use crate::errors::{FLVError, FLVResult};

use super::{EncryptionFilterParams, EncryptionTagHeader, SelectiveEncryptionFilterParams};

impl EncryptionTagHeader {
    pub fn read_from<R>(mut reader: R) -> FLVResult<EncryptionTagHeader>
    where
        R: io::Read,
    {
        let num_filters = reader.read_u8()?;
        let name = amf::amf0::Value::read_from(reader.by_ref())?;
        if name.is_none() {
            return Err(FLVError::UnexpectedValue(
                "expect string for encryption tag header filter name but got none".to_string(),
            ));
        }

        let name = name.expect("this cannot be none");
        let filter_name;
        match name {
            amf::amf0::Value::String(str) => filter_name = str,
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

impl EncryptionFilterParams {
    pub fn read_from<R>(mut reader: R) -> FLVResult<Self>
    where
        R: io::Read,
    {
        let mut iv = [0; 16];
        reader.read_exact(&mut iv)?;
        Ok(Self { iv })
    }
}

impl SelectiveEncryptionFilterParams {
    pub fn read_from<R>(mut reader: R) -> FLVResult<Self>
    where
        R: io::Read,
    {
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
