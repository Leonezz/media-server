use crate::chunk::errors::ChunkMessageError;

use super::{
    AbortMessage, Acknowledgement, ProtocolControlMessage, ProtocolControlMessageType,
    SetChunkSize, SetPeerBandwidth, WindowAckSize, consts::MAX_CHUNK_SIZE,
};
use byteorder::{BigEndian, ReadBytesExt};
use std::io;
use utils::traits::reader::{ReadFrom, ReadRemainingFrom};

impl<R: io::Read> ReadRemainingFrom<ProtocolControlMessageType, R> for ProtocolControlMessage {
    type Error = ChunkMessageError;
    fn read_remaining_from(
        header: ProtocolControlMessageType,
        reader: &mut R,
    ) -> Result<Self, Self::Error> {
        match header {
            ProtocolControlMessageType::SetChunkSize => Ok(ProtocolControlMessage::SetChunkSize(
                SetChunkSize::read_from(reader)?,
            )),
            ProtocolControlMessageType::Abort => Ok(ProtocolControlMessage::Abort(
                AbortMessage::read_from(reader)?,
            )),
            ProtocolControlMessageType::Acknowledgement => Ok(ProtocolControlMessage::Ack(
                Acknowledgement::read_from(reader)?,
            )),
            ProtocolControlMessageType::WindowAckSize => Ok(ProtocolControlMessage::WindowAckSize(
                WindowAckSize::read_from(reader)?,
            )),
            ProtocolControlMessageType::SetPeerBandwidth => Ok(
                ProtocolControlMessage::SetPeerBandwidth(SetPeerBandwidth::read_from(reader)?),
            ),
        }
    }
}

impl<R: io::Read> ReadFrom<R> for SetChunkSize {
    type Error = ChunkMessageError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let chunk_size = reader.read_u32::<BigEndian>()?;
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
}

impl<R: io::Read> ReadFrom<R> for AbortMessage {
    type Error = ChunkMessageError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        Ok(AbortMessage {
            chunk_stream_id: reader.read_u32::<BigEndian>()?,
        })
    }
}

impl<R: io::Read> ReadFrom<R> for Acknowledgement {
    type Error = ChunkMessageError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        Ok(Acknowledgement {
            sequence_number: reader.read_u32::<BigEndian>()?,
        })
    }
}

impl<R: io::Read> ReadFrom<R> for WindowAckSize {
    type Error = ChunkMessageError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        Ok(WindowAckSize {
            size: reader.read_u32::<BigEndian>()?,
        })
    }
}

impl<R: io::Read> ReadFrom<R> for SetPeerBandwidth {
    type Error = ChunkMessageError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let size = reader.read_u32::<BigEndian>()?;
        let limit_type = reader.read_u8()?;

        Ok(SetPeerBandwidth {
            size,
            limit_type: limit_type.try_into()?,
        })
    }
}
