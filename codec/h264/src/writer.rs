use std::io;

use crate::{errors::H264CodecError, nalu::NalUnit};
use byteorder::WriteBytesExt;
use utils::traits::writer::WriteTo;

impl<W: io::Write> WriteTo<W> for NalUnit {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u8(self.header.into())?;
        writer.write_all(&self.body)?;
        Ok(())
    }
}
