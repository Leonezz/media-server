use std::io::{Cursor, Read};

use tokio_util::{
    bytes::{Buf, BufMut},
    codec::{Decoder, Encoder},
};
use utils::traits::{reader::TryReadFrom, writer::WriteTo};

use crate::errors::RtpError;

use super::RtpTrivialPacket;

#[derive(Debug)]
pub struct RtpTrivialPacketFramed;

impl Encoder<RtpTrivialPacket> for RtpTrivialPacketFramed {
    type Error = RtpError;
    fn encode(
        &mut self,
        item: RtpTrivialPacket,
        dst: &mut tokio_util::bytes::BytesMut,
    ) -> Result<(), Self::Error> {
        let mut bytes_writer = dst.writer();
        item.write_to(&mut bytes_writer)
    }
}

impl Decoder for RtpTrivialPacketFramed {
    type Error = RtpError;
    type Item = RtpTrivialPacket;
    fn decode(
        &mut self,
        src: &mut tokio_util::bytes::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        let (res, position) = {
            let mut cursor = Cursor::new(&src);
            let res = RtpTrivialPacket::try_read_from(cursor.by_ref());
            (res, cursor.position())
        };
        if let Ok(Some(_)) = &res {
            src.advance(position as usize);
        }
        res
    }
}
