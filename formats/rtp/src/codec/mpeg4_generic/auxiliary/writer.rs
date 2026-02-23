use crate::codec::mpeg4_generic::{errors::RtpMpeg4Error, parameters::rfc3640::RtpMpeg4Fmtp};
use bitstream_io::{BigEndian, BitWrite, BitWriter};
use num::ToPrimitive;
use std::io;
use utils::traits::writer::WriteTo;

use super::AuxiliaryData;

pub struct AuxiliaryDataWriteWrapper<'a>(pub &'a AuxiliaryData, pub &'a RtpMpeg4Fmtp);

impl<'a, W: io::Write> WriteTo<W> for AuxiliaryDataWriteWrapper<'a> {
    type Error = RtpMpeg4Error;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let (value, param) = (&self.0, &self.1);
        let auxiliary_data_size_length = param.auxiliary_data_size_length.unwrap_or(0);
        if auxiliary_data_size_length == 0 {
            return Err(RtpMpeg4Error::AuxiliaryDataEmpty);
        }

        let mut writer = BitWriter::endian(writer, BigEndian);
        writer.write_var(
            auxiliary_data_size_length.to_u32().unwrap(),
            value.auxiliary_data_size,
        )?;
        writer.write_bytes(&value.data)?;
        writer.byte_align()?;
        Ok(())
    }
}
