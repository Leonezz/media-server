use crate::chunk::errors::{ChunkMessageError, ChunkMessageResult};

use super::{
    AbortMessage, Acknowledgement, ProtocolControlMessage, SetChunkSize, SetPeerBandwidth,
    WindowAckSize, consts::MAX_CHUNK_SIZE,
};
use byteorder::{BigEndian, WriteBytesExt};
use std::io;

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

    pub fn write(&mut self, message: &ProtocolControlMessage) -> ChunkMessageResult<()> {
        match message {
            ProtocolControlMessage::SetChunkSize(m) => self.write_set_chunk_size_message(m),
            ProtocolControlMessage::Abort(m) => self.write_abort_message(m),
            ProtocolControlMessage::Ack(m) => self.write_acknowledgement_message(m),
            ProtocolControlMessage::WindowAckSize(m) => self.write_window_ack_size_message(m),
            ProtocolControlMessage::SetPeerBandwidth(m) => self.write_set_peer_bandwidth_message(m),
        }
    }

    fn write_set_chunk_size_message(&mut self, message: &SetChunkSize) -> ChunkMessageResult<()> {
        if (message.chunk_size as i32) < 0 {
            return Err(ChunkMessageError::InvalidMessage(format!(
                "invalid set chunk size message, the first bit of chunk size is not 0"
            )));
        }

        self.inner
            .write_u32::<BigEndian>(message.chunk_size.min(MAX_CHUNK_SIZE))?;
        Ok(())
    }

    fn write_abort_message(&mut self, message: &AbortMessage) -> ChunkMessageResult<()> {
        self.inner.write_u32::<BigEndian>(message.chunk_stream_id)?;
        Ok(())
    }

    fn write_acknowledgement_message(
        &mut self,
        message: &Acknowledgement,
    ) -> ChunkMessageResult<()> {
        self.inner.write_u32::<BigEndian>(message.sequence_number)?;
        Ok(())
    }

    fn write_window_ack_size_message(&mut self, message: &WindowAckSize) -> ChunkMessageResult<()> {
        self.inner.write_u32::<BigEndian>(message.size)?;
        Ok(())
    }

    fn write_set_peer_bandwidth_message(
        &mut self,
        message: &SetPeerBandwidth,
    ) -> ChunkMessageResult<()> {
        self.inner.write_u32::<BigEndian>(message.size)?;
        self.inner.write_u8(message.limit_type as u8)?;
        Ok(())
    }
}
