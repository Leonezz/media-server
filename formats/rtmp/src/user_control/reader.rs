use std::io;

use byteorder::{BigEndian, ReadBytesExt};

use crate::chunk::errors::ChunkMessageResult;

use super::{UserControlEvent, UserControlEventType};

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

    pub fn read(&mut self) -> ChunkMessageResult<UserControlEvent> {
        let event_type = self.inner.read_u16::<BigEndian>()?;
        let event_type: UserControlEventType = event_type.try_into()?;
        match event_type {
            UserControlEventType::StreamBegin => self.read_stream_begin(),
            UserControlEventType::StreamEOF => self.read_stream_eof(),
            UserControlEventType::SetBufferLength => self.read_set_buffer_length(),
            UserControlEventType::StreamDry => self.read_stream_dry(),
            UserControlEventType::StreamIdsRecorded => self.read_stream_ids_recorded(),
            UserControlEventType::PingRequest => self.read_ping_request(),
            UserControlEventType::PingResponse => self.read_ping_response(),
        }
    }

    fn read_stream_begin(&mut self) -> ChunkMessageResult<UserControlEvent> {
        Ok(UserControlEvent::StreamBegin {
            stream_id: self.inner.read_u32::<BigEndian>()?,
        })
    }

    fn read_stream_eof(&mut self) -> ChunkMessageResult<UserControlEvent> {
        Ok(UserControlEvent::StreamEOF {
            stream_id: self.inner.read_u32::<BigEndian>()?,
        })
    }

    fn read_stream_dry(&mut self) -> ChunkMessageResult<UserControlEvent> {
        Ok(UserControlEvent::StreamDry {
            stream_id: self.inner.read_u32::<BigEndian>()?,
        })
    }

    fn read_set_buffer_length(&mut self) -> ChunkMessageResult<UserControlEvent> {
        Ok(UserControlEvent::SetBufferLength {
            stream_id: self.inner.read_u32::<BigEndian>()?,
            buffer_length: self.inner.read_u32::<BigEndian>()?,
        })
    }

    fn read_stream_ids_recorded(&mut self) -> ChunkMessageResult<UserControlEvent> {
        Ok(UserControlEvent::StreamIdsRecorded {
            stream_id: self.inner.read_u32::<BigEndian>()?,
        })
    }

    fn read_ping_request(&mut self) -> ChunkMessageResult<UserControlEvent> {
        Ok(UserControlEvent::PingRequest {
            timestamp: self.inner.read_u32::<BigEndian>()?,
        })
    }

    fn read_ping_response(&mut self) -> ChunkMessageResult<UserControlEvent> {
        Ok(UserControlEvent::PingResponse {
            timestamp: self.inner.read_u32::<BigEndian>()?,
        })
    }
}
