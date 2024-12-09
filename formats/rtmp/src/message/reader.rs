use crate::{chunk::errors::ChunkMessageResult, commands, user_control};

use super::{RtmpMessage, RtmpMessageHeader, RtmpMessageType, RtmpUserMessageBody};
use byteorder::{BigEndian, ReadBytesExt};
use std::io;
use tokio_util::bytes::{Buf, BytesMut};

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

    pub fn read_c2s(&mut self, version: amf::Version) -> ChunkMessageResult<RtmpMessage> {
        let message_header = self.read_message_header()?;
        let mut payload = BytesMut::with_capacity(message_header.payload_length as usize);
        payload.resize(message_header.payload_length as usize, 0);
        self.inner.read_exact(&mut payload)?;

        let message = match message_header.message_type {
            RtmpMessageType::UserControl => RtmpUserMessageBody::UserControl(
                user_control::UserControlEvent::read_from(payload.reader())?,
            ),
            RtmpMessageType::AMF0Data | RtmpMessageType::AMF3Data => {
                RtmpUserMessageBody::MetaData(amf::Value::read_from(payload.reader(), version)?)
            }
            RtmpMessageType::Audio => RtmpUserMessageBody::Audio { payload },
            RtmpMessageType::Video => RtmpUserMessageBody::Video { payload },
            RtmpMessageType::Aggregate => RtmpUserMessageBody::Aggregate { payload },
            RtmpMessageType::AMF0Command | RtmpMessageType::AMF3Command => {
                RtmpUserMessageBody::C2SCommand(commands::RtmpC2SCommands::read_from(
                    payload.reader(),
                    version,
                )?)
            }
            RtmpMessageType::AMF0SharedObject | RtmpMessageType::AMF3SharedObject => {
                todo!("no spec on this")
            }
        };

        Ok(RtmpMessage {
            header: message_header,
            message,
        })
    }

    //TODO - S2C ?

    fn read_message_header(&mut self) -> ChunkMessageResult<RtmpMessageHeader> {
        let message_type = self.inner.read_u8()?;
        let message_type: RtmpMessageType = message_type.try_into()?;
        let payload_length = self.inner.read_u24::<BigEndian>()?;
        let timestamp = self.inner.read_u32::<BigEndian>()?;
        let stream_id = self.inner.read_u24::<BigEndian>()?;
        Ok(RtmpMessageHeader {
            message_type,
            payload_length,
            timestamp,
            stream_id,
        })
    }
}
