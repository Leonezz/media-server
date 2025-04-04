use std::{
    fmt::{Debug, Write},
    io::{self, Read},
};

use tokio_util::{
    bytes::Buf,
    codec::{Decoder, Encoder},
};
use utils::traits::reader::TryReadFrom;

use crate::errors::RtspMessageError;

use super::RtspRequest;

#[derive(Debug)]
pub struct RtspRequestFramed;

impl Encoder<RtspRequest> for RtspRequestFramed {
    type Error = RtspMessageError;

    fn encode(
        &mut self,
        item: RtspRequest,
        dst: &mut tokio_util::bytes::BytesMut,
    ) -> Result<(), Self::Error> {
        dst.write_fmt(format_args!("{}", item))?;
        Ok(())
    }
}

impl Decoder for RtspRequestFramed {
    type Error = RtspMessageError;
    type Item = RtspRequest;

    fn decode(
        &mut self,
        src: &mut tokio_util::bytes::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        let (res, position) = {
            let mut cursor = io::Cursor::new(&src);
            let res = RtspRequest::try_read_from(cursor.by_ref());
            (res, cursor.position())
        };
        if res.is_ok() {
            src.advance(position as usize);
        }
        res
    }
}
