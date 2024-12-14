use crate::errors::AmfError;
use tokio_util::{
    bytes::{Buf, BufMut, BytesMut},
    codec::{Decoder, Encoder},
};

use super::{Reader, Value, Writer};

pub struct Amf0ValueCodec;

impl Decoder for Amf0ValueCodec {
    type Error = AmfError;
    type Item = Value;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let bytes_reader = src.reader();
        Reader::new(bytes_reader).read()
    }
}

impl Encoder<Value> for Amf0ValueCodec {
    type Error = AmfError;
    fn encode(&mut self, item: Value, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let bytes_writer = dst.writer();
        Writer::new(bytes_writer).write(&item)?;
        Ok(())
    }
}
