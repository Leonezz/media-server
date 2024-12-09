use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::{collections::HashMap, io};
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

pub struct Reader<R> {
    inner: R,
    context: ChunkStreamReadContext,
}

impl<R> Reader<R>
where
    R: io::Read,
{
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            context: HashMap::new(),
        }
    }
    pub fn read(&mut self, c2s: bool) -> ChunkMessageResult<ChunkMessage> {
        let common_header = self.read_to_common_header()?;
        let message_length = common_header.message_length as usize;
        let mut bytes = BytesMut::with_capacity(message_length);
        bytes.resize(message_length, 0);
        self.inner.read_exact(&mut bytes)?;
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
                    RtmpChunkMessageBody::RtmpUserMessage(message::RtmpMessage::read_c2s_from(
                        bytes.reader(),
                        amf::Version::Amf0,
                    )?)
                } else {
                    RtmpChunkMessageBody::RtmpUserMessage(message::RtmpMessage::read_s2c_from(
                        bytes.reader(),
                        amf::Version::Amf0,
                    )?)
                }
            }
        };
        Ok(ChunkMessage {
            header: common_header,
            chunk_message_body: message_body,
        })
    }

    fn read_to_common_header(&mut self) -> ChunkMessageResult<ChunkMessageCommonHeader> {
        let basic_header = self.read_basic_header()?;
        let fmt = basic_header.fmt;
        let message_header = self.read_message_header(fmt)?;
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
                    let timestamp_delta = self.inner.read_u32::<BigEndian>()?;
                    context.timestamp_delta = timestamp_delta;
                    context.timestamp += timestamp_delta;
                } else {
                    context.timestamp += context.timestamp_delta;
                }
            }
        }

        Ok(ChunkMessageCommonHeader {
            basic_header,
            timestamp: context.timestamp,
            message_length: context.message_length,
            message_type_id: context.message_type_id,
            message_stream_id: context.message_stream_id,
        })
    }

    fn read_basic_header(&mut self) -> ChunkMessageResult<ChunkBasicHeader> {
        let first_byte = self.inner.read_u8()?;
        let fmt = first_byte >> 6;
        let maybe_csid = first_byte & 0b00111111;
        match maybe_csid {
            0 => {
                let csid = self.inner.read_u8()?;
                return Ok(ChunkBasicHeader {
                    header_type: super::ChunkBasicHeaderType::Type2,
                    fmt,
                    chunk_stream_id: csid as CSID + 64,
                });
            }
            1 => {
                let csid = self.inner.read_u16::<BigEndian>()?;
                return Ok(ChunkBasicHeader {
                    header_type: super::ChunkBasicHeaderType::Type3,
                    fmt,
                    chunk_stream_id: csid as CSID + 64,
                });
            }
            csid => {
                return Ok(ChunkBasicHeader {
                    header_type: super::ChunkBasicHeaderType::Type1,
                    fmt,
                    chunk_stream_id: csid as CSID,
                });
            }
        }
    }

    fn read_message_header(&mut self, fmt: u8) -> ChunkMessageResult<ChunkMessageHeader> {
        match fmt {
            0 => Ok(ChunkMessageHeader::Type0(self.read_message_header_type0()?)),
            1 => Ok(ChunkMessageHeader::Type1(self.read_message_header_type1()?)),
            2 => Ok(ChunkMessageHeader::Type2(self.read_message_header_type2()?)),
            3 => Ok(ChunkMessageHeader::Type3(ChunkMessageHeaderType3 {})),
            _ => Err(super::errors::ChunkMessageError::UnexpectedFmt(fmt)),
        }
    }

    fn read_message_header_type0(&mut self) -> ChunkMessageResult<ChunkMessageHeaderType0> {
        let mut header0 = ChunkMessageHeaderType0 {
            timestamp: self.inner.read_u24::<BigEndian>()?,
            message_length: self.inner.read_u24::<BigEndian>()?,
            message_type_id: self.inner.read_u8()?,
            message_stream_id: self.inner.read_u32::<LittleEndian>()?,
        };
        if header0.timestamp >= MAX_TIMESTAMP {
            header0.timestamp = self.inner.read_u32::<BigEndian>()?;
        }
        Ok(header0)
    }

    fn read_message_header_type1(&mut self) -> ChunkMessageResult<ChunkMessageHeaderType1> {
        let mut header1 = ChunkMessageHeaderType1 {
            timestamp_delta: self.inner.read_u24::<BigEndian>()?,
            message_length: self.inner.read_u24::<BigEndian>()?,
            message_type_id: self.inner.read_u8()?,
        };
        if header1.timestamp_delta >= MAX_TIMESTAMP {
            header1.timestamp_delta = self.inner.read_u32::<BigEndian>()?;
        }
        Ok(header1)
    }

    fn read_message_header_type2(&mut self) -> ChunkMessageResult<ChunkMessageHeaderType2> {
        let mut header2 = ChunkMessageHeaderType2 {
            timestamp_delta: self.inner.read_u24::<BigEndian>()?,
        };
        if header2.timestamp_delta >= MAX_TIMESTAMP {
            header2.timestamp_delta = self.inner.read_u32::<BigEndian>()?;
        }
        Ok(header2)
    }
}
