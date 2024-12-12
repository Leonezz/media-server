use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::{
    collections::HashMap,
    io::{self, Cursor, Read},
};
use tokio_util::bytes::{Buf, BytesMut};

use crate::{message, protocol_control};

use super::{
    CSID, ChunkBasicHeader, ChunkMessage, ChunkMessageCommonHeader, ChunkMessageHeader,
    ChunkMessageHeaderType0, ChunkMessageHeaderType1, ChunkMessageHeaderType2,
    ChunkMessageHeaderType3, ChunkMessageType, RtmpChunkMessageBody, consts::MAX_TIMESTAMP,
    errors::ChunkMessageResult,
};

#[derive(Debug, Default)]
pub struct ReadContext {
    timestamp: u32,
    timestamp_delta: u32,
    extended_timestamp_enabled: bool,
    message_length: u32,
    message_stream_id: u32,
    message_type_id: u8,
}

type ChunkStreamReadContext = HashMap<CSID, ReadContext>;

#[derive(Debug)]
pub struct Reader {
    context: ChunkStreamReadContext,
}

impl Reader {
    pub fn new() -> Self {
        Self {
            context: HashMap::new(),
        }
    }
    pub fn read(
        &mut self,
        reader: &mut Cursor<&[u8]>,
        c2s: bool,
    ) -> ChunkMessageResult<Option<ChunkMessage>> {
        let common_header = self.read_to_common_header(reader)?;
        if common_header.is_none() {
            return Ok(None);
        }
        let common_header = common_header.expect("this cannot be none");

        let message_length = common_header.message_length as usize;

        if reader.remaining() < message_length {
            return Ok(None);
        }
        let mut bytes = BytesMut::with_capacity(message_length);
        bytes.resize(message_length, 0);
        reader.read_exact(&mut bytes)?;
        let message_body = match common_header.message_type_id.try_into()? {
            ChunkMessageType::ProtocolControl(message_type) => {
                RtmpChunkMessageBody::ProtocolControl(
                    protocol_control::ProtocolControlMessage::read_from(
                        bytes.reader(),
                        message_type,
                    )?,
                )
            }
            ChunkMessageType::RtmpUserMessage(message_type) => {
                if c2s {
                    RtmpChunkMessageBody::RtmpUserMessage(
                        message::RtmpUserMessageBody::read_c2s_from(
                            bytes.reader(),
                            amf::Version::Amf0,
                            &common_header,
                        )?,
                    )
                } else {
                    RtmpChunkMessageBody::RtmpUserMessage(
                        message::RtmpUserMessageBody::read_s2c_from(
                            bytes.reader(),
                            amf::Version::Amf0,
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

    fn read_to_common_header(
        &mut self,
        reader: &mut Cursor<&[u8]>,
    ) -> ChunkMessageResult<Option<ChunkMessageCommonHeader>> {
        let basic_header = self.read_basic_header(reader)?;
        if basic_header.is_none() {
            return Ok(None);
        }
        let basic_header = basic_header.expect("this cannot be none");

        let fmt = basic_header.fmt;

        let message_header = self.read_message_header(reader, fmt)?;
        if message_header.is_none() {
            return Ok(None);
        }
        let message_header = message_header.expect("this cannot be none");

        let csid = basic_header.chunk_stream_id.clone();
        if !self.context.contains_key(&csid) {
            match message_header {
                ChunkMessageHeader::Type0(_) => {
                    let res = self.context.insert(csid, ReadContext::default());
                    if res.is_none() {
                        return Err(super::errors::ChunkMessageError::AddContextFailed);
                    }
                }
                _ => return Err(super::errors::ChunkMessageError::NeedContext),
            }
        }

        let context = self
            .context
            .get_mut(&csid)
            .expect(format!("the context map should have this key: {}", csid).as_str());
        match &message_header {
            ChunkMessageHeader::Type0(header0) => {
                context.message_length = header0.message_length;
                context.message_type_id = header0.message_type_id;
                context.timestamp = header0.timestamp;
                context.extended_timestamp_enabled = header0.timestamp >= MAX_TIMESTAMP;
                context.message_stream_id = header0.message_stream_id;
                context.timestamp_delta = 0;
            }
            ChunkMessageHeader::Type1(header1) => {
                context.message_length = header1.message_length;
                context.message_type_id = header1.message_type_id;
                context.timestamp_delta = header1.timestamp_delta;
                context.timestamp += header1.timestamp_delta;
                context.extended_timestamp_enabled = header1.timestamp_delta >= MAX_TIMESTAMP;
            }
            ChunkMessageHeader::Type2(header2) => {
                context.timestamp_delta = header2.timestamp_delta;
                context.timestamp += header2.timestamp_delta;
                context.extended_timestamp_enabled = header2.timestamp_delta >= MAX_TIMESTAMP;
            }
            ChunkMessageHeader::Type3(_) => {
                //TODO - better check this out
                if context.extended_timestamp_enabled {
                    if reader.remaining() < 4 {
                        return Ok(None);
                    }
                    let timestamp_delta = reader.read_u32::<BigEndian>()?;
                    context.timestamp_delta = timestamp_delta;
                    context.timestamp += timestamp_delta;
                } else {
                    context.timestamp += context.timestamp_delta;
                }
            }
        }

        Ok(Some(ChunkMessageCommonHeader {
            basic_header,
            timestamp: context.timestamp,
            message_length: context.message_length,
            message_type_id: context.message_type_id,
            message_stream_id: context.message_stream_id,
        }))
    }

    fn read_basic_header(
        &mut self,
        reader: &mut Cursor<&[u8]>,
    ) -> ChunkMessageResult<Option<ChunkBasicHeader>> {
        if !reader.has_remaining() {
            return Ok(None);
        }
        let first_byte = reader.read_u8()?;
        let fmt = first_byte >> 6 & 0b11;
        let maybe_csid = first_byte & 0b00111111;
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
                let csid = reader.read_u16::<BigEndian>()?;
                return Ok(Some(ChunkBasicHeader {
                    header_type: super::ChunkBasicHeaderType::Type3,
                    fmt,
                    chunk_stream_id: csid as CSID + 64,
                }));
            }
            csid => {
                return Ok(Some(ChunkBasicHeader {
                    header_type: super::ChunkBasicHeaderType::Type1,
                    fmt,
                    chunk_stream_id: csid as CSID,
                }));
            }
        }
    }

    fn read_message_header(
        &mut self,
        reader: &mut Cursor<&[u8]>,
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
        reader: &mut Cursor<&[u8]>,
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
        reader: &mut Cursor<&[u8]>,
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
        reader: &mut Cursor<&[u8]>,
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
