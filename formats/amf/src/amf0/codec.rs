

use tokio_util::{bytes::{Buf, BufMut, BytesMut}, codec::{Decoder, Encoder}};
use crate::errors::{AmfReadError, AmfWriteError};

use super::{Reader, Value, Writer};

pub struct Amf0ValueCodec;

impl Decoder for Amf0ValueCodec {
    type Error = AmfReadError;
    type Item = Value;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let bytes_reader = src.reader();
        Ok(Some(Reader::new(bytes_reader).read()?))
    }
}

impl Encoder<Value> for Amf0ValueCodec {
    type Error = AmfWriteError;
    fn encode(&mut self, item: Value, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let bytes_writer = dst.writer();
        Writer::new(bytes_writer).write(&item)?;
        Ok(())
    }
}