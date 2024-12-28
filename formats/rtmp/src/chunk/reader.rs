use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::{
    cmp::min,
    collections::HashMap,
    io::{Cursor, Read},
};
use tokio_util::bytes::{Buf, BytesMut};
use utils::system::time::get_timestamp_ns;

use crate::{
    chunk::errors::ChunkMessageError,
    message::{self, RtmpMessageType},
    protocol_control, user_control,
};

use super::{
    CSID, ChunkBasicHeader, ChunkBasicHeaderType, ChunkMessage, ChunkMessageCommonHeader,
    ChunkMessageHeader, ChunkMessageHeaderType0, ChunkMessageHeaderType1, ChunkMessageHeaderType2,
    ChunkMessageHeaderType3, ChunkMessageType, RtmpChunkMessageBody, RuntimeStat,
    consts::MAX_TIMESTAMP, errors::ChunkMessageResult,
};

#[derive(Debug, Default)]
pub struct ChunkPayload {
    pub payload: BytesMut,
    pub total_length: usize,
    pub remaining_length: usize,
}

#[derive(Debug, Default)]
pub struct ReadContext {
    timestamp: u64,
    timestamp_delta: u64,
    extended_timestamp_enabled: bool,
    message_length: u32,
    message_stream_id: u32,
    message_type_id: u8,
    pub incomplete_chunk: Option<ChunkPayload>,
}

type ChunkStreamReadContext = HashMap<CSID, ReadContext>;

#[derive(Debug)]
pub struct Reader {
    context: ChunkStreamReadContext,
    chunk_size: usize,
    bytes_received: u32,
}

impl Reader {
    pub fn new() -> Self {
        Self {
            context: HashMap::new(),
            chunk_size: 128,
            bytes_received: 0,
        }
    }

    #[inline]
    pub fn get_bytes_read(&self) -> u32 {
        self.bytes_received
    }

    pub fn set_chunk_size(&mut self, size: usize) -> usize {
        let old_size = self.chunk_size;
        self.chunk_size = size;
        old_size
    }
    pub fn abort_chunk_message(&mut self, csid: u32) {
        match self.context.get_mut(&csid) {
            None => {}
            Some(ctx) => ctx.incomplete_chunk = None,
        }
    }
    pub fn read(
        &mut self,
        reader: &mut Cursor<&BytesMut>,
        c2s: bool,
    ) -> ChunkMessageResult<Option<ChunkMessage>> {
        let common_header = self.read_to_common_header(reader)?;
        if common_header.is_none() {
            return Ok(None);
        }
        let common_header = common_header.expect("this cannot be none");

        let bytes = self.read_chunk_body(reader, common_header.basic_header.chunk_stream_id)?;
        if bytes.is_none() {
            return Ok(None);
        } else {
            // reset incomplete chunk after a full read
            self.context
                .get_mut(&common_header.basic_header.chunk_stream_id)
                .expect("this cannot be none")
                .incomplete_chunk = None
        }

        let bytes = bytes.expect("this cannot be none");

        let mut bytes_read = match common_header.basic_header.header_type {
            ChunkBasicHeaderType::Type1 => 1,
            ChunkBasicHeaderType::Type2 => 2,
            ChunkBasicHeaderType::Type3 => 3,
        };
        bytes_read += match common_header.basic_header.fmt {
            0 => 11,
            1 => 7,
            2 => 3,
            _ => 0,
        };
        if common_header.extended_timestamp_enabled {
            bytes_read += 4;
        }

        if self.bytes_received + bytes_read > 0xF000_0000 {
            self.bytes_received = bytes_read;
        } else {
            self.bytes_received += bytes_read;
        }

        let message_body = match common_header.message_type_id.try_into()? {
            ChunkMessageType::ProtocolControl(message_type) => {
                RtmpChunkMessageBody::ProtocolControl(
                    protocol_control::ProtocolControlMessage::read_from(&bytes[..], message_type)?,
                )
            }
            ChunkMessageType::UserControl => RtmpChunkMessageBody::UserControl(
                user_control::UserControlEvent::read_from(&bytes[..])?,
            ),
            ChunkMessageType::RtmpUserMessage(message_type) => {
                if c2s {
                    RtmpChunkMessageBody::RtmpUserMessage(
                        message::RtmpUserMessageBody::read_c2s_from(
                            bytes.reader(),
                            match message_type {
                                RtmpMessageType::AMF3Command
                                | RtmpMessageType::AMF3Data
                                | RtmpMessageType::AMF3SharedObject => amf::Version::Amf3,
                                _ => amf::Version::Amf0,
                            },
                            &common_header,
                        )?,
                    )
                } else {
                    RtmpChunkMessageBody::RtmpUserMessage(
                        message::RtmpUserMessageBody::read_s2c_from(
                            bytes.reader(),
                            match message_type {
                                RtmpMessageType::AMF3Command
                                | RtmpMessageType::AMF3Data
                                | RtmpMessageType::AMF3SharedObject => amf::Version::Amf3,
                                _ => amf::Version::Amf0,
                            },
                            &common_header,
                        )?,
                    )
                }
            }
        };

        Ok(Some(ChunkMessage {
            header: common_header,
            chunk_message_body: message_body,
        }))
    }

    fn read_chunk_body(
        &mut self,
        reader: &mut Cursor<&BytesMut>,
        csid: u32,
    ) -> ChunkMessageResult<Option<BytesMut>> {
        let ctx = self.context.get_mut(&csid);
        if ctx.is_none() {
            return Err(ChunkMessageError::NeedContext);
        }

        let ctx = ctx.expect("this cannot be none");

        if ctx.incomplete_chunk.is_none() {
            ctx.incomplete_chunk = Some(ChunkPayload {
                payload: BytesMut::with_capacity(ctx.message_length as usize),
                total_length: ctx.message_length as usize,
                remaining_length: ctx.message_length as usize,
            })
        }

        let chunk = ctx.incomplete_chunk.as_mut().expect("this cannot be none");

        let bytes_need = min(self.chunk_size, chunk.remaining_length);
        if reader.remaining() < bytes_need {
            return Ok(None);
        }

        let mut bytes = Vec::with_capacity(bytes_need);
        bytes.resize(bytes_need, 0);
        reader.read_exact(&mut bytes)?;

        chunk.remaining_length -= bytes_need;
        chunk.payload.extend_from_slice(&bytes);

        if chunk.remaining_length == 0 {
            return Ok(Some(chunk.payload.clone()));
        }

        Err(ChunkMessageError::IncompleteChunk)
    }

    fn read_to_common_header(
        &mut self,
        reader: &mut Cursor<&BytesMut>,
    ) -> ChunkMessageResult<Option<ChunkMessageCommonHeader>> {
        let basic_header = self.read_basic_header(reader)?;
        if basic_header.is_none() {
            return Ok(None);
        }
        let basic_header = basic_header.expect("this cannot be none");

        let fmt = basic_header.fmt;
        let message_header = self.read_message_header(reader, fmt)?;

        let csid = basic_header.chunk_stream_id.clone();
        if !self.context.contains_key(&csid) {
            if fmt != 0 {
                tracing::error!(
                    "new chunk must start with a type 0 message header, {:?}, {:?}",
                    basic_header,
                    message_header
                );
                // return Err(ChunkMessageError::NeedContext);
            }
            self.context.insert(csid, ReadContext::default());
        }

        if message_header.is_none() {
            return Ok(None);
        }
        let message_header = message_header.expect("this cannot be none");

        let context = self
            .context
            .get_mut(&csid)
            .expect(format!("the context map should have this key: {}", csid).as_str());
        match &message_header {
            ChunkMessageHeader::Type0(header0) => {
                context.message_length = header0.message_length;
                context.message_type_id = header0.message_type_id;
                context.timestamp = header0.timestamp as u64;
                context.extended_timestamp_enabled = header0.timestamp >= MAX_TIMESTAMP;
                context.message_stream_id = header0.message_stream_id;
                context.timestamp_delta = 0;
            }
            ChunkMessageHeader::Type1(header1) => {
                context.message_length = header1.message_length;
                context.message_type_id = header1.message_type_id;
                context.timestamp_delta = header1.timestamp_delta as u64;
                context.timestamp += header1.timestamp_delta as u64;
                context.extended_timestamp_enabled = header1.timestamp_delta >= MAX_TIMESTAMP;
            }
            ChunkMessageHeader::Type2(header2) => {
                context.timestamp_delta = header2.timestamp_delta as u64;
                context.timestamp += header2.timestamp_delta as u64;
                context.extended_timestamp_enabled = header2.timestamp_delta >= MAX_TIMESTAMP;
            }
            ChunkMessageHeader::Type3(_) => {
                //TODO - better check this out
                if context.extended_timestamp_enabled {
                    if reader.remaining() < 4 {
                        return Ok(None);
                    }
                    let timestamp_delta = reader.read_u32::<BigEndian>()?;
                    context.timestamp_delta = timestamp_delta as u64;
                    context.timestamp += timestamp_delta as u64;
                } else {
                    context.timestamp += context.timestamp_delta;
                }
            }
        }

        Ok(Some(ChunkMessageCommonHeader {
            basic_header,
            timestamp: context.timestamp as u32,
            message_length: context.message_length,
            message_type_id: context.message_type_id,
            message_stream_id: context.message_stream_id,
            extended_timestamp_enabled: context.extended_timestamp_enabled,
            runtime_stat: RuntimeStat {
                read_time_ns: get_timestamp_ns().unwrap_or(0),
                ..Default::default()
            },
        }))
    }

    fn read_basic_header(
        &mut self,
        reader: &mut Cursor<&BytesMut>,
    ) -> ChunkMessageResult<Option<ChunkBasicHeader>> {
        if !reader.has_remaining() {
            return Ok(None);
        }

        let first_byte = reader.read_u8()?;

        let fmt = (first_byte >> 6) & 0b11;
        let maybe_csid = (first_byte & 0b00111111) as u32;
        match maybe_csid {
            0 => {
                if !reader.has_remaining() {
                    return Ok(None);
                }
                let csid = reader.read_u8()?;

                return Ok(Some(ChunkBasicHeader {
                    header_type: super::ChunkBasicHeaderType::Type2,
                    fmt,
                    chunk_stream_id: csid as CSID + 64,
                }));
            }
            1 => {
                if reader.remaining() < 2 {
                    return Ok(None);
                }
                let mut csid = 64;
                csid += reader.read_u8()? as u32;
                csid += reader.read_u8()? as u32 * 256;

                return Ok(Some(ChunkBasicHeader {
                    header_type: super::ChunkBasicHeaderType::Type3,
                    fmt,
                    chunk_stream_id: csid,
                }));
            }
            csid => {
                return Ok(Some(ChunkBasicHeader {
                    header_type: super::ChunkBasicHeaderType::Type1,
                    fmt,
                    chunk_stream_id: csid,
                }));
            }
        }
    }

    fn read_message_header(
        &mut self,
        reader: &mut Cursor<&BytesMut>,
        fmt: u8,
    ) -> ChunkMessageResult<Option<ChunkMessageHeader>> {
        match fmt {
            0 => {
                if reader.remaining() < 11 {
                    return Ok(None);
                } else {
                    return Ok(Some(ChunkMessageHeader::Type0(
                        self.read_message_header_type0(reader)?,
                    )));
                }
            }
            1 => {
                if reader.remaining() < 7 {
                    return Ok(None);
                } else {
                    return Ok(Some(ChunkMessageHeader::Type1(
                        self.read_message_header_type1(reader)?,
                    )));
                }
            }
            2 => {
                if reader.remaining() < 3 {
                    return Ok(None);
                } else {
                    return Ok(Some(ChunkMessageHeader::Type2(
                        self.read_message_header_type2(reader)?,
                    )));
                }
            }
            3 => Ok(Some(ChunkMessageHeader::Type3(ChunkMessageHeaderType3 {}))),
            _ => Err(super::errors::ChunkMessageError::UnexpectedFmt(fmt)),
        }
    }

    fn read_message_header_type0(
        &mut self,
        reader: &mut Cursor<&BytesMut>,
    ) -> ChunkMessageResult<ChunkMessageHeaderType0> {
        let mut header0 = ChunkMessageHeaderType0 {
            timestamp: reader.read_u24::<BigEndian>()?,
            message_length: reader.read_u24::<BigEndian>()?,
            message_type_id: reader.read_u8()?,
            message_stream_id: reader.read_u32::<LittleEndian>()?,
        };
        if header0.timestamp >= MAX_TIMESTAMP {
            header0.timestamp = reader.read_u32::<BigEndian>()?;
        }
        Ok(header0)
    }

    fn read_message_header_type1(
        &mut self,
        reader: &mut Cursor<&BytesMut>,
    ) -> ChunkMessageResult<ChunkMessageHeaderType1> {
        let mut header1 = ChunkMessageHeaderType1 {
            timestamp_delta: reader.read_u24::<BigEndian>()?,
            message_length: reader.read_u24::<BigEndian>()?,
            message_type_id: reader.read_u8()?,
        };
        if header1.timestamp_delta >= MAX_TIMESTAMP {
            header1.timestamp_delta = reader.read_u32::<BigEndian>()?;
        }
        Ok(header1)
    }

    fn read_message_header_type2(
        &mut self,
        reader: &mut Cursor<&BytesMut>,
    ) -> ChunkMessageResult<ChunkMessageHeaderType2> {
        let mut header2 = ChunkMessageHeaderType2 {
            timestamp_delta: reader.read_u24::<BigEndian>()?,
        };
        if header2.timestamp_delta >= MAX_TIMESTAMP {
            header2.timestamp_delta = reader.read_u32::<BigEndian>()?;
        }
        Ok(header2)
    }
}
