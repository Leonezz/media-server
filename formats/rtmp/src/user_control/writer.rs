use std::io;

use crate::chunk::errors::ChunkMessageError;

use super::{UserControlEvent, UserControlEventType};
use byteorder::{BigEndian, WriteBytesExt};
use utils::traits::writer::WriteTo;

impl<W: io::Write> WriteTo<W> for UserControlEvent {
    type Error = ChunkMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        match self {
            UserControlEvent::StreamBegin { stream_id } => {
                writer.write_u16::<BigEndian>(UserControlEventType::StreamBegin.into())?;
                writer.write_u32::<BigEndian>(*stream_id)?;
                Ok(())
            }
            UserControlEvent::StreamEOF { stream_id } => {
                writer.write_u16::<BigEndian>(UserControlEventType::StreamEOF.into())?;
                writer.write_u32::<BigEndian>(*stream_id)?;
                Ok(())
            }
            UserControlEvent::StreamDry { stream_id } => {
                writer.write_u16::<BigEndian>(UserControlEventType::StreamDry.into())?;
                writer.write_u32::<BigEndian>(*stream_id)?;
                Ok(())
            }
            UserControlEvent::SetBufferLength {
                stream_id,
                buffer_length,
            } => {
                writer.write_u16::<BigEndian>(UserControlEventType::SetBufferLength.into())?;
                writer.write_u32::<BigEndian>(*stream_id)?;
                writer.write_u32::<BigEndian>(*buffer_length)?;
                Ok(())
            }
            UserControlEvent::StreamIdsRecorded { stream_id } => {
                writer.write_u16::<BigEndian>(UserControlEventType::StreamIdsRecorded.into())?;
                writer.write_u32::<BigEndian>(*stream_id)?;
                Ok(())
            }
            UserControlEvent::PingRequest { timestamp } => {
                writer.write_u16::<BigEndian>(UserControlEventType::PingRequest.into())?;
                writer.write_u32::<BigEndian>(*timestamp)?;
                Ok(())
            }
            UserControlEvent::PingResponse { timestamp } => {
                writer.write_u16::<BigEndian>(UserControlEventType::PingResponse.into())?;
                writer.write_u32::<BigEndian>(*timestamp)?;
                Ok(())
            }
        }
    }
}
