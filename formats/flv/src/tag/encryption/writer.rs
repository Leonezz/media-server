use byteorder::{BigEndian, WriteBytesExt};

use std::io;

use crate::errors::FLVResult;

use super::{EncryptionFilterParams, EncryptionTagHeader, SelectiveEncryptionFilterParams};
impl EncryptionTagHeader {
    pub fn write_to<W>(&self, mut writer: W) -> FLVResult<()>
    where
        W: io::Write,
    {
        writer.write_u8(self.num_filters)?;
        amf::amf0::Value::String(self.filter_name.clone()).write_to(writer.by_ref())?;
        writer.write_u24::<BigEndian>(self.length)?;
        Ok(())
    }
}

impl EncryptionFilterParams {
    pub fn write_to<W>(&self, mut writer: W) -> FLVResult<()>
    where
        W: io::Write,
    {
        writer.write_all(&self.iv)?;
        Ok(())
    }
}

impl SelectiveEncryptionFilterParams {
    pub fn write_to<W>(&self, mut writer: W) -> FLVResult<()>
    where
        W: io::Write,
    {
        if let Some(iv) = self.iv {
            writer.write_u8(0b1000_0000)?;
            writer.write_all(&iv)?;
        } else {
            writer.write_u8(0)?;
        }
        Ok(())
    }
}
