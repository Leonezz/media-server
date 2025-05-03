use std::io;

use bitstream_io::{BigEndian, BitWrite2, BitWriter};
use num::ToPrimitive;
use utils::traits::writer::WriteTo;

use crate::codec::mpeg4_generic::{
    errors::{RtpMpeg4Error, RtpMpeg4Result},
    parameters::RtpMpeg4OutOfBandParams,
};

use super::{AuHeader, AuHeaderSection};

pub struct AuHeaderWriteWrapper<'a>(pub &'a RtpMpeg4OutOfBandParams, pub bool, pub &'a AuHeader);
impl<'a> AuHeaderWriteWrapper<'a> {
    pub fn write_to<W: BitWrite2>(&self, writer: &mut W) -> RtpMpeg4Result<()> {
        let (param, is_first, value) = (&self.0, self.1, &self.2);
        if let Some(size_length) = param.size_length
            && size_length > 0
            && let Some(au_size) = value.au_size
        {
            bitstream_io::BitWrite2::write(
                writer,
                size_length.to_u32().expect("integer overflow u32"),
                au_size,
            )?;
        }

        if is_first {
            if let Some(au_index_length) = param.index_length
                && au_index_length > 0
                && let Some(au_index) = value.au_index
            {
                bitstream_io::BitWrite2::write(
                    writer,
                    au_index_length.to_u32().expect("integer overflow u32"),
                    au_index,
                )?;
            }
        } else if let Some(au_index_delta_length) = param.index_delta_length
            && au_index_delta_length > 0
            && let Some(au_index_delta) = value.au_index_delta
        {
            bitstream_io::BitWrite2::write(
                writer,
                au_index_delta_length
                    .to_u32()
                    .expect("integer overflow u32"),
                au_index_delta,
            )?;
        }

        if let Some(cts_delta_length) = param.cts_delta_length
            && cts_delta_length > 0
        {
            if is_first {
                writer.write_bit(false)?;
            } else if let Some(cts_delta) = value.cts_delta {
                writer.write_bit(true)?;
                bitstream_io::BitWrite2::write(
                    writer,
                    cts_delta_length.to_u32().expect("integer overflow u32"),
                    cts_delta,
                )?;
            } else {
                writer.write_bit(false)?;
            }
        }

        if let Some(dts_delta_length) = param.dts_delta_length
            && dts_delta_length > 0
        {
            if let Some(dts_delta) = value.dts_delta {
                writer.write_bit(true)?;
                bitstream_io::BitWrite2::write(
                    writer,
                    dts_delta_length.to_u32().expect("integer overflow u32"),
                    dts_delta,
                )?;
            } else {
                writer.write_bit(false)?;
            }
        }

        if param.random_access_indication.unwrap_or(false) {
            writer.write_bit(value.rap_flag.unwrap_or(false))?;
        }

        if let Some(stream_state_indicator) = param.stream_state_indication
            && stream_state_indicator > 0
            && let Some(stream_state) = value.stream_state
        {
            bitstream_io::BitWrite2::write(
                writer,
                stream_state_indicator
                    .to_u32()
                    .expect("integer overflow u32"),
                stream_state,
            )?;
        }

        Ok(())
    }
}

pub struct AuHeaderSectionWriteWrapper<'a>(
    pub &'a AuHeaderSection,
    pub &'a RtpMpeg4OutOfBandParams,
);

impl<'a, W: io::Write> WriteTo<W> for AuHeaderSectionWriteWrapper<'a> {
    type Error = RtpMpeg4Error;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let (value, param) = (&self.0, &self.1);
        let mut writer = BitWriter::endian(writer, BigEndian);
        bitstream_io::write::BitWrite2::write(&mut writer, 16, value.au_headers_length)?;
        let mut is_first = true;
        for header in &value.au_headers {
            let write_wapper = &AuHeaderWriteWrapper(param, is_first, header);
            write_wapper.write_to(&mut writer)?;
            is_first = false;
        }
        bitstream_io::write::BitWrite2::byte_align(&mut writer)?;
        Ok(())
    }
}
