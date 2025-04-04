use std::io::{self, Read};

use tokio_util::{
    bytes::{Buf, BufMut},
    codec::{Decoder, Encoder},
};
use utils::traits::{reader::TryReadFrom, writer::WriteTo};

use crate::errors::RtspMessageError;

use super::RtspInterleavedPacket;

#[derive(Debug)]
pub struct RtspInterleavedPacketFramed;

impl Encoder<RtspInterleavedPacket> for RtspInterleavedPacketFramed {
    type Error = RtspMessageError;
    fn encode(
        &mut self,
        item: RtspInterleavedPacket,
        dst: &mut tokio_util::bytes::BytesMut,
    ) -> Result<(), Self::Error> {
        item.write_to(dst.writer())
    }
}

impl Decoder for RtspInterleavedPacketFramed {
    type Error = RtspMessageError;
    type Item = RtspInterleavedPacket;

    fn decode(
        &mut self,
        src: &mut tokio_util::bytes::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        let (res, position) = {
            let mut cursor = io::Cursor::new(&src);
            let res = RtspInterleavedPacket::try_read_from(cursor.by_ref());
            (res, cursor.position())
        };
        if res.is_ok() {
            src.advance(position as usize);
        }
        res
    }
}
