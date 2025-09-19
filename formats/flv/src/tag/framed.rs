use std::io::{self, Read};

use tokio_util::{
    bytes::{Buf, BufMut},
    codec::{Decoder, Encoder},
};
use utils::traits::{reader::TryReadFrom, writer::WriteTo};

use crate::errors::FLVError;

use super::FLVTag;

pub struct FLVTagFramed;

impl Encoder<FLVTag> for FLVTagFramed {
    type Error = FLVError;
    fn encode(
        &mut self,
        item: FLVTag,
        dst: &mut tokio_util::bytes::BytesMut,
    ) -> Result<(), Self::Error> {
        item.write_to(&mut dst.writer())?;
        Ok(())
    }
}

impl Decoder for FLVTagFramed {
    type Error = FLVError;
    type Item = FLVTag;
    fn decode(
        &mut self,
        src: &mut tokio_util::bytes::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        let (res, position) = {
            let mut cursor = io::Cursor::new(&src);
            let res = FLVTag::try_read_from(cursor.by_ref());
            (res, cursor.position())
        };
        if res.is_ok() && res.as_ref().unwrap().is_some() {
            src.advance(position as usize);
        }
        res
    }
}
