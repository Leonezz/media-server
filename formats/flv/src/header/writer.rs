use std::io;

use byteorder::{BigEndian, WriteBytesExt};

use crate::errors::FLVResult;

use super::FLVHeader;

#[derive(Debug)]
pub struct Writer<W> {
    inner: W,
}

impl<W> Writer<W>
where
    W: io::Write,
{
    pub fn new(inner: W) -> Self {
        Self { inner }
    }

    pub fn write(&mut self, header: &FLVHeader) -> FLVResult<()> {
        self.inner.write_all(&header.flv_marker)?;
        self.inner.write_u8(header.flv_version)?;

        let mut byte: u8 = 0;

        byte |= (header.has_audio as u8) << 2;
        byte |= (header.has_video as u8) << 0;

        self.inner.write_u8(byte)?;
        self.inner.write_u32::<BigEndian>(header.data_offset)?;
        Ok(())
    }
}
