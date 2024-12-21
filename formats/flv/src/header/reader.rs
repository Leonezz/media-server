use crate::errors::{FLVError, FLVResult};
use byteorder::{BigEndian, ReadBytesExt};
use std::io;

use super::FLVHeader;

#[derive(Debug)]
pub struct Reader<R> {
    inner: R,
}

impl<R> Reader<R>
where
    R: io::Read,
{
    pub fn new(inner: R) -> Self {
        Self { inner }
    }

    pub fn read(&mut self) -> FLVResult<FLVHeader> {
        let mut signature = [0; 3];
        self.inner.read_exact(&mut signature)?;
        if signature != [b'F', b'L', b'V'] {
            return Err(FLVError::UnknownSignature(signature));
        }

        let version = self.inner.read_u8()?;

        let byte = self.inner.read_u8()?;

        let data_offset = self.inner.read_u32::<BigEndian>()?;

        Ok(FLVHeader {
            flv_marker: signature,
            flv_version: version,
            has_audio: (byte & 0b100) != 0,
            has_video: (byte & 0b1) != 0,
            data_offset,
        })
    }
}
