use std::io;

use bitstream_io::{BigEndian, BitRead2, BitReader};
use num::ToPrimitive;
use utils::traits::reader::ReadRemainingFrom;

use crate::codec::mpeg4_generic::{
    errors::{RtpMpeg4Error, RtpMpeg4Result},
    parameters::RtpMpeg4Fmtp,
};

use super::{AuHeader, AuHeaderSection};

impl AuHeader {
    pub fn read_remaining_from<R: BitRead2>(
        header: (&RtpMpeg4Fmtp, bool),
        mut reader: R,
    ) -> RtpMpeg4Result<Self> {
        let (param, is_first) = header;
        let mut result = Self::default();
        let mut bits_cnt = 0;
        if let Some(size_length) = param.size_length
            && size_length > 0
        {
            result.au_size = Some(bitstream_io::BitRead2::read(
                &mut reader,
                size_length.to_u32().expect("integer overflow u32"),
            )?);
            bits_cnt += size_length;
        }
        if is_first {
            if let Some(au_index_length) = param.index_length
                && au_index_length > 0
            {
                let au_index = bitstream_io::BitRead2::read(
                    &mut reader,
                    au_index_length.to_u32().expect("integer overflow u32"),
                )?;
                result.au_index = Some(au_index);
                bits_cnt += au_index_length;
            }
        } else if let Some(au_index_delta_length) = param.index_delta_length
            && au_index_delta_length > 0
        {
            result.au_index_delta = Some(bitstream_io::BitRead2::read(
                &mut reader,
                au_index_delta_length
                    .to_u32()
                    .expect("integer overflow u32"),
            )?);
            bits_cnt += au_index_delta_length;
        }

        if let Some(cts_delta_length) = param.cts_delta_length
            && cts_delta_length > 0
        {
            let cts_flag = reader.read_bit()?;
            bits_cnt += 1;
            if cts_flag && is_first {
                return Err(RtpMpeg4Error::SyntaxError(
                    "got cts_flag being true with the first au".to_owned(),
                ));
            }
            if cts_flag {
                result.cts_delta = Some(bitstream_io::BitRead2::read(
                    &mut reader,
                    cts_delta_length.to_u32().expect("integer overflow u32"),
                )?);
                bits_cnt += cts_delta_length;
            }
        }

        if let Some(dts_delta_length) = param.dts_delta_length
            && dts_delta_length > 0
        {
            let dts_flag = reader.read_bit()?;
            bits_cnt += 1;
            if dts_flag {
                result.dts_delta = Some(bitstream_io::BitRead2::read(
                    &mut reader,
                    dts_delta_length.to_u32().expect("integer overflow u32"),
                )?);
                bits_cnt += dts_delta_length;
            }
        }

        if let Some(rap_indicator) = param.random_access_indication
            && rap_indicator
        {
            result.rap_flag = Some(reader.read_bit()?);
            bits_cnt += 1;
        }

        if let Some(stream_state_indcator) = param.stream_state_indication {
            result.stream_state = Some(bitstream_io::BitRead2::read(
                &mut reader,
                stream_state_indcator
                    .to_u32()
                    .expect("integer overflow u32"),
            )?);
            bits_cnt += stream_state_indcator;
        }

        result.bits_cnt = bits_cnt;
        Ok(result)
    }
}

impl<R: io::Read> ReadRemainingFrom<&RtpMpeg4Fmtp, R> for AuHeaderSection {
    type Error = RtpMpeg4Error;
    fn read_remaining_from(header: &RtpMpeg4Fmtp, reader: &mut R) -> Result<Self, Self::Error> {
        let mut reader = BitReader::endian(reader, BigEndian);
        let au_headers_length = reader.read_in::<16, u64>()?;
        let mut headers = vec![];
        let mut bits_read = 0;
        while bits_read < au_headers_length {
            let header = AuHeader::read_remaining_from((header, bits_read == 0), &mut reader)?;
            bits_read += header.bits_cnt;
            headers.push(header);
        }

        bitstream_io::read::BitRead2::byte_align(&mut reader);

        Ok(Self {
            au_headers_length,
            au_headers: headers,
        })
    }
}
