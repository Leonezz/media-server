use crate::chunk::errors::ChunkMessageError;

use super::{
    AbortMessage, Acknowledgement, ProtocolControlMessage, SetChunkSize, SetPeerBandwidth,
    WindowAckSize, consts::MAX_CHUNK_SIZE,
};
use byteorder::{BigEndian, WriteBytesExt};
use std::io;
use utils::traits::writer::WriteTo;

impl<W: io::Write> WriteTo<W> for ProtocolControlMessage {
    type Error = ChunkMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        match self {
            ProtocolControlMessage::SetChunkSize(m) => m.write_to(writer),
            ProtocolControlMessage::Abort(m) => m.write_to(writer),
            ProtocolControlMessage::Ack(m) => m.write_to(writer),
            ProtocolControlMessage::WindowAckSize(m) => m.write_to(writer),
            ProtocolControlMessage::SetPeerBandwidth(m) => m.write_to(writer),
        }
    }
}

impl<W: io::Write> WriteTo<W> for SetChunkSize {
    type Error = ChunkMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        if (self.chunk_size as i32) < 0 {
            return Err(ChunkMessageError::InvalidMessage(
                "invalid set chunk size message, the first bit of chunk size is not 0".to_owned(),
            ));
        }

        writer.write_u32::<BigEndian>(self.chunk_size.min(MAX_CHUNK_SIZE))?;
        Ok(())
    }
}

impl<W: io::Write> WriteTo<W> for AbortMessage {
    type Error = ChunkMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u32::<BigEndian>(self.chunk_stream_id)?;
        Ok(())
    }
}

impl<W: io::Write> WriteTo<W> for Acknowledgement {
    type Error = ChunkMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u32::<BigEndian>(self.sequence_number)?;
        Ok(())
    }
}

impl<W: io::Write> WriteTo<W> for WindowAckSize {
    type Error = ChunkMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u32::<BigEndian>(self.size)?;
        Ok(())
    }
}

impl<W: io::Write> WriteTo<W> for SetPeerBandwidth {
    type Error = ChunkMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u32::<BigEndian>(self.size)?;
        writer.write_u8(self.limit_type as u8)?;
        Ok(())
    }
}
