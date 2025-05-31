use std::io;

use crate::{
    errors::H264CodecError,
    nalu::NalUnit,
    rbsp::{need_escape, rbsp_to_sodb},
};
use byteorder::WriteBytesExt;
use utils::traits::writer::WriteTo;

impl<W: io::Write> WriteTo<W> for NalUnit {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u8(self.header.into())?;
        if need_escape(&self.body) {
            let escaped = rbsp_to_sodb(&self.body);
            writer.write_all(&escaped)?;
        } else {
            writer.write_all(&self.body)?;
        }
        Ok(())
    }
}
