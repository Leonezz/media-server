use std::io;

use bitstream_io::{BigEndian, BitWrite, BitWriter};
use num::ToPrimitive;
use utils::traits::writer::{BitwiseWriteTo, WriteTo};

use crate::codec::mpeg4_generic::{errors::RtpMpeg4Error, parameters::RtpMpeg4Fmtp};

use super::{AuHeader, AuHeaderSection};

pub struct AuHeaderWriteWrapper<'a>(pub &'a RtpMpeg4Fmtp, pub bool, pub &'a AuHeader);
impl<'a, W: BitWrite> BitwiseWriteTo<W> for AuHeaderWriteWrapper<'a> {
    type Error = RtpMpeg4Error;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let (param, is_first, value) = (&self.0, self.1, &self.2);
        if let Some(size_length) = param.size_length
            && size_length > 0
            && let Some(au_size) = value.au_size
        {
            writer.write_var(size_length.to_u32().unwrap(), au_size)?;
        }

        if is_first {
            if let Some(au_index_length) = param.index_length
                && au_index_length > 0
                && let Some(au_index) = value.au_index
            {
                writer.write_var(au_index_length.to_u32().unwrap(), au_index)?;
            }
        } else if let Some(au_index_delta_length) = param.index_delta_length
            && au_index_delta_length > 0
            && let Some(au_index_delta) = value.au_index_delta
        {
            writer.write_var(au_index_delta_length.to_u32().unwrap(), au_index_delta)?;
        }

        if let Some(cts_delta_length) = param.cts_delta_length
            && cts_delta_length > 0
        {
            if is_first {
                writer.write_bit(false)?;
            } else if let Some(cts_delta) = value.cts_delta {
                writer.write_bit(true)?;
                writer.write_var(cts_delta_length.to_u32().unwrap(), cts_delta)?;
            } else {
                writer.write_bit(false)?;
            }
        }

        if let Some(dts_delta_length) = param.dts_delta_length
            && dts_delta_length > 0
        {
            if let Some(dts_delta) = value.dts_delta {
                writer.write_bit(true)?;
                writer.write_var(dts_delta_length.to_u32().unwrap(), dts_delta)?;
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
            writer.write_var(stream_state_indicator.to_u32().unwrap(), stream_state)?;
        }

        Ok(())
    }
}

pub struct AuHeaderSectionWriteWrapper<'a>(pub &'a AuHeaderSection, pub &'a RtpMpeg4Fmtp);

impl<'a, W: io::Write> WriteTo<W> for AuHeaderSectionWriteWrapper<'a> {
    type Error = RtpMpeg4Error;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let (value, param) = (&self.0, &self.1);
        let mut writer = BitWriter::endian(writer, BigEndian);
        writer.write::<16, u64>(value.au_headers_length)?;
        let mut is_first = true;
        for header in &value.au_headers {
            let write_wapper = &AuHeaderWriteWrapper(param, is_first, header);
            write_wapper.write_to(&mut writer)?;
            is_first = false;
        }
        writer.byte_align()?;
        Ok(())
    }
}
