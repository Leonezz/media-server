use std::io;

use super::RtmpUserMessageBody;
use crate::chunk::errors::{ChunkMessageError, ChunkMessageResult};

#[derive(Debug)]
pub struct Writer<W> {
    inner: W,
}

impl<W> Writer<W>
where
    W: io::Write,
{
    pub fn new(inner: W) -> Self {
        Self { inner }
    }

    pub fn write(
        &mut self,
        message: &RtmpUserMessageBody,
        version: amf::Version,
    ) -> ChunkMessageResult<()> {
        match message {
            RtmpUserMessageBody::C2SCommand(command) => {
                command.write_to(self.inner.by_ref(), version)
            }
            RtmpUserMessageBody::S2Command(command) => {
                command.write_to(self.inner.by_ref(), version)
            }
            RtmpUserMessageBody::MetaData(data) => data
                .write_to(self.inner.by_ref())
                .map_err(|err| ChunkMessageError::MetaDataError(err)),
            RtmpUserMessageBody::Audio { payload }
            | RtmpUserMessageBody::Video { payload }
            | RtmpUserMessageBody::Aggregate { payload } => self
                .inner
                .write_all(&payload)
                .map_err(|err| ChunkMessageError::Io(err)),
            RtmpUserMessageBody::SharedObject() => todo!(),
        }
    }

    // fn write_header(&mut self, header: &RtmpMessageHeader) -> ChunkMessageResult<()> {
    //     self.inner.write_u8(header.message_type.into())?;
    //     self.inner.write_u24::<BigEndian>(header.payload_length)?;
    //     self.inner.write_u32::<BigEndian>(header.timestamp)?;
    //     self.inner.write_u24::<BigEndian>(header.stream_id)?;
    //     Ok(())
    // }
}
