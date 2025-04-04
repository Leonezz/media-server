use std::{fmt::Write, io::Read};

use tokio_util::{
    bytes::Buf,
    codec::{Decoder, Encoder},
};
use utils::traits::reader::TryReadFrom;

use crate::errors::RtspMessageError;

use super::RtspResponse;

#[derive(Debug)]
pub struct RtspResponseFramed;

impl Encoder<RtspResponse> for RtspResponseFramed {
    type Error = RtspMessageError;
    fn encode(
        &mut self,
        item: RtspResponse,
        dst: &mut tokio_util::bytes::BytesMut,
    ) -> Result<(), Self::Error> {
        dst.write_fmt(format_args!("{}", item))?;
        Ok(())
    }
}

impl Decoder for RtspResponseFramed {
    type Error = RtspMessageError;
    type Item = RtspResponse;
    fn decode(
        &mut self,
        src: &mut tokio_util::bytes::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        let (res, position) = {
            let mut cursor = std::io::Cursor::new(&src);
            let res = RtspResponse::try_read_from(cursor.by_ref());
            (res, cursor.position())
        };
        if res.is_ok() {
            src.advance(position as usize);
        }
        res
    }
}
