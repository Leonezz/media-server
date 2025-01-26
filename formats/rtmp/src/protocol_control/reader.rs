use crate::chunk::errors::{ChunkMessageError, ChunkMessageResult};

use super::{
    AbortMessage, Acknowledgement, ProtocolControlMessage, ProtocolControlMessageType,
    SetChunkSize, SetPeerBandwidth, WindowAckSize, consts::MAX_CHUNK_SIZE,
};
use byteorder::{BigEndian, ReadBytesExt};
use std::io;

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

    pub fn read(
        &mut self,
        message_type_id: ProtocolControlMessageType,
    ) -> ChunkMessageResult<ProtocolControlMessage> {
        match message_type_id {
            ProtocolControlMessageType::SetChunkSize => Ok(ProtocolControlMessage::SetChunkSize(
                self.read_set_chunk_size_message()?,
            )),
            ProtocolControlMessageType::Abort => {
                Ok(ProtocolControlMessage::Abort(self.read_abort_message()?))
            }
            ProtocolControlMessageType::Acknowledgement => Ok(ProtocolControlMessage::Ack(
                self.read_acknowledgement_message()?,
            )),
            ProtocolControlMessageType::WindowAckSize => Ok(ProtocolControlMessage::WindowAckSize(
                self.read_window_ack_size_message()?,
            )),
            ProtocolControlMessageType::SetPeerBandwidth => Ok(
                ProtocolControlMessage::SetPeerBandwidth(self.read_set_peer_bandwidth_message()?),
            ),
        }
    }

    fn read_set_chunk_size_message(&mut self) -> ChunkMessageResult<SetChunkSize> {
        let chunk_size = self.inner.read_u32::<BigEndian>()?;
        if (chunk_size as i32) < 0 {
            return Err(ChunkMessageError::InvalidMessage(format!(
                "invalid set chunk size message, the first bit of chunk size is not zero, chunk size bits: {:#b}",
                chunk_size
            )));
        }
        if chunk_size < 1 {
            return Err(ChunkMessageError::InvalidMessage(
                "invalid set chunk size message, the chunk size is 0".to_owned(),
            ));
        }

        Ok(SetChunkSize {
            chunk_size: chunk_size.min(MAX_CHUNK_SIZE),
        })
    }

    fn read_abort_message(&mut self) -> ChunkMessageResult<AbortMessage> {
        Ok(AbortMessage {
            chunk_stream_id: self.inner.read_u32::<BigEndian>()?,
        })
    }

    fn read_acknowledgement_message(&mut self) -> ChunkMessageResult<Acknowledgement> {
        Ok(Acknowledgement {
            sequence_number: self.inner.read_u32::<BigEndian>()?,
        })
    }

    fn read_window_ack_size_message(&mut self) -> ChunkMessageResult<WindowAckSize> {
        Ok(WindowAckSize {
            size: self.inner.read_u32::<BigEndian>()?,
        })
    }

    fn read_set_peer_bandwidth_message(&mut self) -> ChunkMessageResult<SetPeerBandwidth> {
        let size = self.inner.read_u32::<BigEndian>()?;
        let limit_type = self.inner.read_u8()?;

        Ok(SetPeerBandwidth {
            size,
            limit_type: limit_type.try_into()?,
        })
    }
}
