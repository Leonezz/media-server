use byteorder::{BigEndian, LittleEndian, WriteBytesExt};
use std::{
    collections::HashMap,
    io::{self, Read},
};
use tokio_util::bytes::{Buf, BytesMut};

use super::{
    CSID, ChunkBasicHeader, ChunkBasicHeaderType, ChunkMessage, ChunkMessageCommonHeader,
    ChunkMessageHeader, consts::MAX_TIMESTAMP, errors::ChunkMessageResult,
};

#[derive(Debug, Default)]
struct WriteContext {
    timestamp: u32,
    timestamp_delta: u32,
    extended_timestamp_enabled: bool,
    message_length: u32,
    message_stream_id: u32,
    message_type_id: u8,
    previous_message_header: Option<ChunkMessageHeader>,
}

type ChunkMessageWriteContext = HashMap<CSID, WriteContext>;

pub struct Writer<W> {
    inner: W,
    context: ChunkMessageWriteContext,
}

impl<W> Writer<W>
where
    W: io::Write,
{
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            context: ChunkMessageWriteContext::new(),
        }
    }

    pub fn transparent_write(
        &mut self,
        basic_header: ChunkBasicHeader,
        message_header: ChunkMessageHeader,
        chunk_data: BytesMut,
    ) -> ChunkMessageResult<()> {
        self.write_basic_header(&basic_header)?;
        self.write_message_header(&message_header, basic_header.chunk_stream_id)?;
        self.write_chunk_data(chunk_data)?;
        Ok(())
    }

    pub fn write(&mut self, value: ChunkMessage) -> ChunkMessageResult<()> {
        self.validate_message(&value)?;

        let (basic_header, message_header) = self.justify_message_type(&value.header)?;
        self.transparent_write(basic_header, message_header, value.chunk_data)?;

        Ok(())
    }

    fn justify_message_type(
        &mut self,
        value: &ChunkMessageCommonHeader,
    ) -> ChunkMessageResult<(ChunkBasicHeader, ChunkMessageHeader)> {
        let basic_header = value.basic_header.clone();

        // no context at all, this must be the first message of this chunk stream
        if !self.context.contains_key(&basic_header.chunk_stream_id) {
            return Ok((
                basic_header,
                ChunkMessageHeader::Type0(super::ChunkMessageHeaderType0 {
                    timestamp: value.timestamp,
                    message_length: value.message_length,
                    message_type_id: value.message_type_id,
                    message_stream_id: value.message_stream_id,
                }),
            ));
        }

        let ctx = self.context.get_mut(&basic_header.chunk_stream_id).expect(
            format!(
                "there should be {} in context map",
                basic_header.chunk_stream_id
            )
            .as_str(),
        );

        // maybe something weird happened, but we don't know what type the last message header is
        if ctx.previous_message_header.is_none() {
            return Ok((
                basic_header,
                ChunkMessageHeader::Type0(super::ChunkMessageHeaderType0 {
                    timestamp: value.timestamp,
                    message_length: value.message_length,
                    message_type_id: value.message_type_id,
                    message_stream_id: value.message_stream_id,
                }),
            ));
        }

        let previous_message_header = ctx
            .previous_message_header
            .as_ref()
            .expect("this must be valid");

        if let ChunkMessageHeader::Type0(_) = previous_message_header {
            // see: 5.3.1.2.4. Type 3
            if value.timestamp == ctx.timestamp * 2 {
                // make sure the timestamp_delta is correct when writing Type3 header
                ctx.timestamp_delta = ctx.timestamp;
                return Ok((
                    basic_header,
                    ChunkMessageHeader::Type3(super::ChunkMessageHeaderType3 {}),
                ));
            }
        }

        if ctx.message_length == value.message_length
            && ctx.message_stream_id == value.message_stream_id
            && ctx.message_type_id == value.message_type_id
            && ctx.timestamp_delta == value.timestamp - ctx.timestamp
        {
            return Ok((
                basic_header,
                ChunkMessageHeader::Type3(super::ChunkMessageHeaderType3 {}),
            ));
        } else if ctx.message_length == value.message_length
            && ctx.message_stream_id == value.message_stream_id
            && ctx.message_type_id == value.message_type_id
        {
            return Ok((
                basic_header,
                ChunkMessageHeader::Type2(super::ChunkMessageHeaderType2 {
                    timestamp_delta: value.timestamp - ctx.timestamp,
                }),
            ));
        } else if ctx.message_stream_id == value.message_stream_id {
            return Ok((
                basic_header,
                ChunkMessageHeader::Type1(super::ChunkMessageHeaderType1 {
                    timestamp_delta: value.timestamp - ctx.timestamp,
                    message_length: value.message_length,
                    message_type_id: value.message_type_id,
                }),
            ));
        } else {
            return Ok((
                basic_header,
                ChunkMessageHeader::Type0(super::ChunkMessageHeaderType0 {
                    timestamp: value.timestamp,
                    message_length: value.message_length,
                    message_type_id: value.message_type_id,
                    message_stream_id: value.message_stream_id,
                }),
            ));
        }
    }

    fn validate_message(&self, value: &ChunkMessage) -> ChunkMessageResult<()> {
        if value.header.message_length as usize != value.chunk_data.remaining() {
            return Err(super::errors::ChunkMessageError::MessageLengthNotMatch);
        }

        Ok(())
    }

    fn write_basic_header(&mut self, header: &ChunkBasicHeader) -> ChunkMessageResult<()> {
        match header.header_type {
            ChunkBasicHeaderType::Type1 => {
                let first_byte = header.fmt << 6 + header.chunk_stream_id;
                self.inner.write_u8(first_byte)?;
            }
            ChunkBasicHeaderType::Type2 => {
                let first_byte = header.fmt << 6;
                self.inner.write_u8(first_byte)?;
                self.inner.write_u8((header.chunk_stream_id - 64) as u8)?;
            }
            ChunkBasicHeaderType::Type3 => {
                let first_byte = header.fmt << 6 | 0b00111111;
                self.inner.write_u8(first_byte)?;
                self.inner
                    .write_u16::<BigEndian>((header.chunk_stream_id - 64) as u16)?;
            }
        }
        Ok(())
    }

    fn write_message_header(
        &mut self,
        header: &ChunkMessageHeader,
        csid: CSID,
    ) -> ChunkMessageResult<()> {
        match header {
            ChunkMessageHeader::Type0(header) => {
                let extended_timestamp_enabled = header.timestamp >= MAX_TIMESTAMP;
                let timestamp_field = header.timestamp.min(MAX_TIMESTAMP);
                self.inner.write_u24::<BigEndian>(timestamp_field)?;
                self.inner.write_u24::<BigEndian>(header.message_length)?;
                self.inner.write_u8(header.message_type_id)?;
                self.inner
                    .write_u32::<LittleEndian>(header.message_stream_id)?;
                if extended_timestamp_enabled {
                    self.inner.write_u32::<BigEndian>(header.timestamp)?;
                }

                if !self.context.contains_key(&csid) {
                    self.context.insert(csid, WriteContext::default());
                }
                let ctx = self
                    .context
                    .get_mut(&csid)
                    .expect(format!("there should be {} in context map", csid).as_str());

                ctx.extended_timestamp_enabled = extended_timestamp_enabled;
                ctx.timestamp = header.timestamp;
                ctx.message_length = header.message_length;
                ctx.message_stream_id = header.message_stream_id;
                ctx.message_type_id = header.message_type_id;
                ctx.timestamp_delta = 0;
            }
            ChunkMessageHeader::Type1(header) => {
                if !self.context.contains_key(&csid) {
                    return Err(super::errors::ChunkMessageError::InvalidMessageHead(
                        format!(
                            "invalid message header, got a type 1 header: {:?} while no context found for csid: {}",
                            header, csid
                        ),
                    ));
                }

                let extended_timestamp_enabled = header.timestamp_delta >= MAX_TIMESTAMP;
                let timestamp_delta_field = header.timestamp_delta.min(MAX_TIMESTAMP);
                self.inner.write_u24::<BigEndian>(timestamp_delta_field)?;
                self.inner.write_u24::<BigEndian>(header.message_length)?;
                self.inner.write_u8(header.message_type_id)?;
                if extended_timestamp_enabled {
                    self.inner.write_u32::<BigEndian>(header.timestamp_delta)?;
                }

                let ctx = self
                    .context
                    .get_mut(&csid)
                    .expect(format!("there should be {} in context map", csid).as_str());

                ctx.extended_timestamp_enabled = extended_timestamp_enabled;
                ctx.timestamp_delta = header.timestamp_delta;
                ctx.timestamp += header.timestamp_delta;
                ctx.message_length = header.message_length;
                ctx.message_type_id = header.message_type_id;
            }
            ChunkMessageHeader::Type2(header) => {
                if !self.context.contains_key(&csid) {
                    return Err(super::errors::ChunkMessageError::InvalidMessageHead(
                        format!(
                            "invalid message header, got a type 2 header: {:?} while no context found for csid: {}",
                            header, csid
                        ),
                    ));
                }

                let extended_timestamp_enabled = header.timestamp_delta >= MAX_TIMESTAMP;
                let timestamp_delta_field = header.timestamp_delta.min(MAX_TIMESTAMP);
                self.inner.write_u24::<BigEndian>(timestamp_delta_field)?;
                if extended_timestamp_enabled {
                    self.inner.write_u32::<BigEndian>(header.timestamp_delta)?;
                }

                let ctx = self
                    .context
                    .get_mut(&csid)
                    .expect(format!("there should be {} in context map", csid).as_str());

                ctx.extended_timestamp_enabled = extended_timestamp_enabled;
                ctx.timestamp_delta = header.timestamp_delta;
                ctx.timestamp += header.timestamp_delta;
            }
            ChunkMessageHeader::Type3(header) => {
                let ctx = self.context.get(&csid);
                if let Some(ctx) = ctx {
                    if ctx.extended_timestamp_enabled {
                        self.inner.write_u32::<BigEndian>(ctx.timestamp_delta)?;
                    }
                } else {
                    return Err(super::errors::ChunkMessageError::InvalidMessageHead(
                        format!(
                            "invalid message header, got a type 3 header: {:?} while no context found for csid: {}",
                            header, csid
                        ),
                    ));
                }
            }
        }
        Ok(())
    }

    fn write_chunk_data(&mut self, bytes: BytesMut) -> ChunkMessageResult<()> {
        let mut buf = Vec::new();
        bytes.reader().read_to_end(&mut buf)?;
        self.inner.write_all(&buf)?;
        Ok(())
    }
}
