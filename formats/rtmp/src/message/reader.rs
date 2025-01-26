use crate::{
    chunk::{ChunkMessageCommonHeader, errors::ChunkMessageResult},
    commands,
};

use super::{RtmpMessageType, RtmpUserMessageBody};
use std::io::{self, Cursor};
use tokio_util::bytes::BytesMut;

#[derive(Debug)]
pub struct Reader<R> {
    inner: R,
}

impl<R> Reader<R>
where
    R: io::Read,
{
    pub fn new(inner: R) -> Self {
        Self { inner }
    }

    pub fn read_c2s(
        &mut self,
        version: amf::Version,
        header: &ChunkMessageCommonHeader,
    ) -> ChunkMessageResult<RtmpUserMessageBody> {
        let mut payload = BytesMut::with_capacity(header.message_length as usize);
        payload.resize(header.message_length as usize, 0);
        self.inner.read_exact(&mut payload)?;

        let payload_reader = Cursor::new(&payload);

        let message = match header.message_type_id.try_into()? {
            RtmpMessageType::AMF0Data | RtmpMessageType::AMF3Data => {
                RtmpUserMessageBody::MetaData { payload }
            }
            RtmpMessageType::Audio => RtmpUserMessageBody::Audio { payload },
            RtmpMessageType::Video => RtmpUserMessageBody::Video { payload },
            RtmpMessageType::Aggregate => RtmpUserMessageBody::Aggregate { payload },
            RtmpMessageType::AMF0Command | RtmpMessageType::AMF3Command => {
                RtmpUserMessageBody::C2SCommand(commands::RtmpC2SCommands::read_from(
                    payload_reader,
                    version,
                )?)
            }
            RtmpMessageType::AMF0SharedObject | RtmpMessageType::AMF3SharedObject => {
                todo!("no spec on this")
            }
        };

        Ok(message)
    }

    //TODO - S2C ?

    // fn read_message_header(&mut self) -> ChunkMessageResult<RtmpMessageHeader> {
    //     let message_type = self.inner.read_u8()?;
    //     let message_type: RtmpMessageType = message_type.try_into()?;
    //     let payload_length = self.inner.read_u24::<BigEndian>()?;
    //     let timestamp = self.inner.read_u32::<BigEndian>()?;
    //     let stream_id = self.inner.read_u24::<BigEndian>()?;
    //     Ok(RtmpMessageHeader {
    //         message_type,
    //         payload_length,
    //         timestamp,
    //         stream_id,
    //     })
    // }
}
