use std::io;

use byteorder::{BigEndian, ReadBytesExt};
use utils::traits::reader::ReadFrom;

use crate::chunk::errors::ChunkMessageError;

use super::{UserControlEvent, UserControlEventType};

impl<R: io::Read> ReadFrom<R> for UserControlEvent {
    type Error = ChunkMessageError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let event_type = reader.read_u16::<BigEndian>()?;
        let event_type: UserControlEventType = event_type.try_into()?;
        match event_type {
            UserControlEventType::StreamBegin => Ok(UserControlEvent::StreamBegin {
                stream_id: reader.read_u32::<BigEndian>()?,
            }),
            UserControlEventType::StreamEOF => Ok(UserControlEvent::StreamEOF {
                stream_id: reader.read_u32::<BigEndian>()?,
            }),
            UserControlEventType::SetBufferLength => Ok(UserControlEvent::SetBufferLength {
                stream_id: reader.read_u32::<BigEndian>()?,
                buffer_length: reader.read_u32::<BigEndian>()?,
            }),
            UserControlEventType::StreamDry => Ok(UserControlEvent::StreamDry {
                stream_id: reader.read_u32::<BigEndian>()?,
            }),
            UserControlEventType::StreamIdsRecorded => Ok(UserControlEvent::StreamIdsRecorded {
                stream_id: reader.read_u32::<BigEndian>()?,
            }),
            UserControlEventType::PingRequest => Ok(UserControlEvent::PingRequest {
                timestamp: reader.read_u32::<BigEndian>()?,
            }),
            UserControlEventType::PingResponse => Ok(UserControlEvent::PingResponse {
                timestamp: reader.read_u32::<BigEndian>()?,
            }),
        }
    }
}
