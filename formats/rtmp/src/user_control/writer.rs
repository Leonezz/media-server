use std::io;

use super::{UserControlEvent, UserControlEventType, errors::errors::RtmpMessageResult};
use byteorder::{BigEndian, WriteBytesExt};

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

    pub fn write(&mut self, event: UserControlEvent) -> RtmpMessageResult<()> {
        match event {
            UserControlEvent::StreamBegin { stream_id } => self.write_stream_begin(stream_id),
            UserControlEvent::StreamEOF { stream_id } => self.write_stream_eof(stream_id),
            UserControlEvent::StreamDry { stream_id } => self.write_stream_dry(stream_id),
            UserControlEvent::SetBufferLength {
                stream_id,
                buffer_length,
            } => self.write_set_buffer_length(stream_id, buffer_length),
            UserControlEvent::StreamIdsRecorded { stream_id } => {
                self.write_stream_ids_recorded(stream_id)
            }
            UserControlEvent::PingRequest { timestamp } => self.write_ping_request(timestamp),
            UserControlEvent::PingResponse { timestamp } => self.write_ping_response(timestamp),
        }
    }

    fn write_stream_begin(&mut self, stream_id: u32) -> RtmpMessageResult<()> {
        self.inner
            .write_u16::<BigEndian>(UserControlEventType::StreamBegin.into())?;
        self.inner.write_u32::<BigEndian>(stream_id)?;
        Ok(())
    }

    fn write_stream_eof(&mut self, stream_id: u32) -> RtmpMessageResult<()> {
        self.inner
            .write_u16::<BigEndian>(UserControlEventType::StreamEOF.into())?;
        self.inner.write_u32::<BigEndian>(stream_id)?;
        Ok(())
    }

    fn write_stream_dry(&mut self, stream_id: u32) -> RtmpMessageResult<()> {
        self.inner
            .write_u16::<BigEndian>(UserControlEventType::StreamDry.into())?;
        self.inner.write_u32::<BigEndian>(stream_id)?;
        Ok(())
    }

    fn write_set_buffer_length(
        &mut self,
        stream_id: u32,
        buffer_length: u32,
    ) -> RtmpMessageResult<()> {
        self.inner
            .write_u16::<BigEndian>(UserControlEventType::SetBufferLength.into())?;
        self.inner.write_u32::<BigEndian>(stream_id)?;
        self.inner.write_u32::<BigEndian>(buffer_length)?;
        Ok(())
    }

    fn write_stream_ids_recorded(&mut self, stream_id: u32) -> RtmpMessageResult<()> {
        self.inner
            .write_u16::<BigEndian>(UserControlEventType::StreamIdsRecorded.into())?;
        self.inner.write_u32::<BigEndian>(stream_id)?;
        Ok(())
    }

    fn write_ping_request(&mut self, timestamp: u32) -> RtmpMessageResult<()> {
        self.inner
            .write_u16::<BigEndian>(UserControlEventType::PingRequest.into())?;
        self.inner.write_u32::<BigEndian>(timestamp)?;
        Ok(())
    }

    fn write_ping_response(&mut self, timestamp: u32) -> RtmpMessageResult<()> {
        self.inner
            .write_u16::<BigEndian>(UserControlEventType::PingResponse.into())?;
        self.inner.write_u32::<BigEndian>(timestamp)?;
        Ok(())
    }
}
