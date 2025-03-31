use std::io::{Cursor, Read};

use tokio_util::{
    bytes::{Buf, BufMut},
    codec::{Decoder, Encoder},
};
use utils::traits::{reader::TryReadFrom, writer::WriteTo};

use crate::errors::RtpError;

use super::compound_packet::RtcpCompoundPacket;

#[derive(Debug)]
pub struct RtcpPacketFramed;

impl Encoder<RtcpCompoundPacket> for RtcpPacketFramed {
    type Error = RtpError;
    fn encode(
        &mut self,
        item: RtcpCompoundPacket,
        dst: &mut tokio_util::bytes::BytesMut,
    ) -> Result<(), Self::Error> {
        let bytes_writer = dst.writer();
        item.write_to(bytes_writer)
    }
}

impl Decoder for RtcpPacketFramed {
    type Error = RtpError;
    type Item = RtcpCompoundPacket;
    fn decode(
        &mut self,
        src: &mut tokio_util::bytes::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        let (res, position) = {
            let mut cursor = Cursor::new(&src);
            let res = RtcpCompoundPacket::try_read_from(cursor.by_ref());
            (res, cursor.position())
        };
        if res.is_ok() {
            src.advance(position as usize);
        }
        res
    }
}
