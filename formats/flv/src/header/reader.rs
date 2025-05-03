use crate::errors::FLVError;
use byteorder::{BigEndian, ReadBytesExt};
use std::io;
use utils::traits::reader::ReadFrom;

use super::FLVHeader;

impl<R: io::Read> ReadFrom<R> for FLVHeader {
    type Error = FLVError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let mut signature = [0; 3];
        reader.read_exact(&mut signature)?;
        if signature != [b'F', b'L', b'V'] {
            return Err(FLVError::UnknownSignature(signature));
        }
        let version = reader.read_u8()?;
        let byte = reader.read_u8()?;
        let data_offset = reader.read_u32::<BigEndian>()?;
        Ok(FLVHeader {
            flv_marker: signature,
            flv_version: version,
            has_audio: (byte & 0b100) != 0,
            has_video: (byte & 0b1) != 0,
            data_offset,
        })
    }
}
