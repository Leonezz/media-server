use std::io;

use byteorder::{BigEndian, WriteBytesExt};
use utils::traits::writer::WriteTo;

use crate::errors::FLVError;

use super::FLVHeader;

impl<W: io::Write> WriteTo<W> for FLVHeader {
    type Error = FLVError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_all(&self.flv_marker)?;
        writer.write_u8(self.flv_version)?;

        let mut byte: u8 = 0;
        byte |= (self.has_audio as u8) << 2;
        byte |= self.has_video as u8;

        writer.write_u8(byte)?;
        writer.write_u32::<BigEndian>(self.data_offset)?;
        Ok(())
    }
}
