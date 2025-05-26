use std::io::{self, Read};

use num::ToPrimitive;
use tokio_util::{
    bytes::{Buf, Bytes, BytesMut},
    either::Either,
};
use utils::traits::reader::ReadRemainingFrom;

use crate::codec::mpeg4_generic::{
    au_header::AuHeader, errors::RtpMpeg4Error, parameters::RtpMpeg4Fmtp,
};

use super::{AccessUnit, AccessUnitFragment, AccessUnitSection};

impl<R: io::Read> ReadRemainingFrom<(&AuHeader, u32, &RtpMpeg4Fmtp), R> for AccessUnit {
    type Error = RtpMpeg4Error;
    fn read_remaining_from(
        header: (&AuHeader, u32, &RtpMpeg4Fmtp),
        reader: &mut R,
    ) -> Result<Self, Self::Error> {
        let (au_header, timestamp, param) = header;
        let bytes_cnt: usize = au_header
            .au_size
            .unwrap_or(param.constant_size.unwrap_or(0))
            .to_usize()
            .expect("integer overflow usize");
        if bytes_cnt == 0 {
            return Err(RtpMpeg4Error::AccessUnitEmpty);
        }

        let mut bytes = vec![0; bytes_cnt];
        reader.read_exact(&mut bytes)?;
        Ok(Self {
            header: au_header.clone(),
            body: Bytes::from_owner(bytes),
            timestamp: timestamp
                .checked_add(
                    au_header
                        .cts_delta
                        .unwrap_or(0)
                        .to_u32()
                        .expect("integer overflow u32"),
                )
                .and_then(|v| {
                    v.checked_add(
                        au_header
                            .au_index_delta
                            .unwrap_or(0)
                            .to_u32()
                            .unwrap()
                            .checked_mul(
                                param
                                    .constant_duration
                                    .unwrap_or(0)
                                    .to_u32()
                                    .expect("integer overflow u32"),
                            )
                            .unwrap(),
                    )
                })
                .expect("timestamp overflow"),
        })
    }
}

impl<R: AsRef<[u8]>> ReadRemainingFrom<(u32, &AuHeader), io::Cursor<R>> for AccessUnitFragment {
    type Error = RtpMpeg4Error;
    fn read_remaining_from(
        header: (u32, &AuHeader),
        reader: &mut io::Cursor<R>,
    ) -> Result<Self, Self::Error> {
        let (timestamp, au_header) = header;

        let mut bytes = BytesMut::zeroed(reader.remaining());
        reader.read_exact(&mut bytes)?;
        Ok(Self {
            header: au_header.clone(),
            body: bytes,
            timestamp: timestamp
                .checked_add(
                    au_header
                        .cts_delta
                        .unwrap_or(0)
                        .to_u32()
                        .expect("cts_delta overflow u32"),
                )
                .expect("timestamp overflow"),
        })
    }
}

impl<R: AsRef<[u8]>> ReadRemainingFrom<(&Vec<AuHeader>, u32, bool, &RtpMpeg4Fmtp), io::Cursor<R>>
    for AccessUnitSection
{
    type Error = RtpMpeg4Error;
    fn read_remaining_from(
        header: (&Vec<AuHeader>, u32, bool, &RtpMpeg4Fmtp),
        reader: &mut io::Cursor<R>,
    ) -> Result<Self, Self::Error> {
        let (au_headers, mut timestamp, is_fragment, params) = header;
        if au_headers.is_empty() {
            return Err(RtpMpeg4Error::AccessUnitEmpty);
        }

        if is_fragment
            || reader.remaining()
                < au_headers[0]
                    .au_size
                    .unwrap_or(0)
                    .to_usize()
                    .expect("au_size overflow usize")
        {
            return Ok(AccessUnitSection {
                access_units_or_fragment: Either::Right(AccessUnitFragment::read_remaining_from(
                    (timestamp, &au_headers[0]),
                    reader,
                )?),
            });
        }

        let mut aus = vec![];
        for au_header in au_headers {
            let au =
                AccessUnit::read_remaining_from((au_header, timestamp, params), reader.by_ref())?;
            timestamp = au.timestamp;
            aus.push(au);
        }
        Ok(Self {
            access_units_or_fragment: Either::Left(aus),
        })
    }
}
