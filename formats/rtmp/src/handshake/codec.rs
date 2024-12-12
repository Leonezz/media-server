use tokio_util::{
    bytes::{Buf, BufMut},
    codec::{Decoder, Encoder},
};

use super::{
    C0S0Packet, C1S1Packet, C2S2Packet, consts::RTMP_HANDSHAKE_SIZE, errors::HandshakeError,
    reader::Reader, writer::Writer,
};

pub struct C0S0PacketCodec;

impl Decoder for C0S0PacketCodec {
    type Error = HandshakeError;
    type Item = C0S0Packet;
    fn decode(
        &mut self,
        src: &mut tokio_util::bytes::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        let len = src.len();
        if len < 1 {
            return Ok(None);
        }

        Ok(Some(Reader::new(src.reader()).read_c0s0()?))
    }
}

impl Encoder<C0S0Packet> for C0S0PacketCodec {
    type Error = HandshakeError;
    fn encode(
        &mut self,
        item: C0S0Packet,
        dst: &mut tokio_util::bytes::BytesMut,
    ) -> Result<(), Self::Error> {
        Writer::new(dst.writer()).write_c0s0(item.version)?;
        Ok(())
    }
}

pub struct C1S1PacketCodec;

impl Decoder for C1S1PacketCodec {
    type Error = HandshakeError;
    type Item = C1S1Packet;
    fn decode(
        &mut self,
        src: &mut tokio_util::bytes::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        let len = src.len();
        if len < RTMP_HANDSHAKE_SIZE {
            src.reserve(RTMP_HANDSHAKE_SIZE);
            return Ok(None);
        }

        Ok(Some(Reader::new(src.reader()).read_c1s1()?))
    }
}

impl Encoder<C1S1Packet> for C1S1PacketCodec {
    type Error = HandshakeError;
    fn encode(
        &mut self,
        item: C1S1Packet,
        dst: &mut tokio_util::bytes::BytesMut,
    ) -> Result<(), Self::Error> {
        dst.reserve(RTMP_HANDSHAKE_SIZE);
        let mut bytes: Vec<u8> = Vec::with_capacity(RTMP_HANDSHAKE_SIZE);
        Writer::new(&mut bytes).write_c1s1(item)?;
        dst.extend_from_slice(&bytes);
        Ok(())
    }
}

pub struct C2S2PacketCodec;

impl Decoder for C2S2PacketCodec {
    type Error = HandshakeError;
    type Item = C2S2Packet;
    fn decode(
        &mut self,
        src: &mut tokio_util::bytes::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        let len = src.len();
        if len < RTMP_HANDSHAKE_SIZE {
            src.reserve(RTMP_HANDSHAKE_SIZE);
            return Ok(None);
        }

        Ok(Some(Reader::new(src.reader()).read_c2s2()?))
    }
}

impl Encoder<C2S2Packet> for C2S2PacketCodec {
    type Error = HandshakeError;
    fn encode(
        &mut self,
        item: C2S2Packet,
        dst: &mut tokio_util::bytes::BytesMut,
    ) -> Result<(), Self::Error> {
        dst.reserve(RTMP_HANDSHAKE_SIZE);
        Writer::new(dst.writer()).write_c2s2(item)?;
        Ok(())
    }
}
