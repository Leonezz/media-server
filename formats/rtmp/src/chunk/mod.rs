use errors::{ChunkMessageError, ChunkMessageResult};
use tokio_util::bytes::BytesMut;

use crate::{
    message::{RtmpMessage, RtmpMessageType},
    protocol_control::{ProtocolControlMessage, ProtocolControlMessageType},
};

pub mod consts;
pub mod errors;
pub mod reader;
pub mod writer;

#[repr(u8)]
#[derive(Debug, Clone)]
enum ChunkBasicHeaderType {
    Type1 = 1,
    Type2 = 2,
    Type3 = 3,
}

type CSID = u32;

///! @see: 5.3.1.1. Chunk Basic Header
///! 1, 2 or 3 bytes
#[derive(Debug, Clone)]
pub struct ChunkBasicHeader {
    header_type: ChunkBasicHeaderType,
    fmt: u8,               // 2 bits
    chunk_stream_id: CSID, // 6 bits / 1 byte / 2 bytes
}

impl ChunkBasicHeader {
    pub fn new(fmt: u8, csid: CSID) -> ChunkMessageResult<Self> {
        let header_type = match csid {
            id if id > 1 && id < 64 => ChunkBasicHeaderType::Type1,
            id if id > 63 && id < 320 => ChunkBasicHeaderType::Type2,
            id if id > 319 && id < 65600 => ChunkBasicHeaderType::Type3,
            _ => {
                return Err(ChunkMessageError::InvalidBasicHeader(format!(
                    "invalid csid: {}",
                    csid
                )));
            }
        };

        Ok(Self {
            header_type,
            fmt,
            chunk_stream_id: csid,
        })
    }
}

///! @see: 5.3.1.2. Chunk Message Header
///! @see: 5.3.1.2.1. Type 0 - for start of a chunk stream, or for timestamp backwards
///! 11 bytes total
///! the timestamp fields must by less or equal then 0xFFFF, and enables extend timestamp field if timestamp equals to 0xFFFF
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                   timestamp                   |message length |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |     message length (cont)     |message type id| msg stream id |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |           message stream id (cont)            |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///                   Chunk Message Header - Type 0
#[derive(Debug, Clone)]
pub struct ChunkMessageHeaderType0 {
    timestamp: u32,         // 3 bytes
    message_length: u32,    // 3 bytes
    message_type_id: u8,    // 1 byte
    message_stream_id: u32, // 4 byte, little endian
}

///! @see: 5.3.1.2.2. Type 1 - this chunk takes the same stream ID as the preceding chunk
///! 7 bytes
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                timestamp delta                |message length |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |     message length (cont)     |message type id|
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///                 Chunk Message Header - Type 1
#[derive(Debug)]
pub struct ChunkMessageHeaderType1 {
    timestamp_delta: u32, // 3 bytes
    message_length: u32,  // 3 bytes
    message_type_id: u8,  // 1 byte
}

///! @see: 5.3.1.2.3. Type 2 - for streams with constant-sized messages
///! 3 bytes
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                timestamp delta                |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///         Chunk Message Header - Type 2
#[derive(Debug)]
pub struct ChunkMessageHeaderType2 {
    timestamp_delta: u32, // 3 bytes
}

///! @see: 5.3.1.2.4. Type 3 - for one message split into multiple chunks
/// there are no message header for this type
#[derive(Debug)]
pub struct ChunkMessageHeaderType3 {}

#[derive(Debug)]
pub enum ChunkMessageHeader {
    Type0(ChunkMessageHeaderType0),
    Type1(ChunkMessageHeaderType1),
    Type2(ChunkMessageHeaderType2),
    Type3(ChunkMessageHeaderType3),
}

#[derive(Debug)]
pub struct ChunkMessageCommonHeader {
    basic_header: ChunkBasicHeader,
    timestamp: u32,
    message_length: u32,
    message_type_id: u8,
    message_stream_id: u32,
}

///! @see: 5.3.1. Chunk Format
/// +--------------+----------------+--------------------+--------------+
/// | Basic Header | Message Header | Extended Timestamp |  Chunk Data  |
/// +--------------+----------------+--------------------+--------------+
/// |                                                    |
/// |<------------------- Chunk Header ----------------->|
///                             Chunk Format
#[derive(Debug)]
pub struct ChunkMessage {
    pub header: ChunkMessageCommonHeader,
    pub chunk_message_body: RtmpChunkMessageBody,
}

#[derive(Debug)]
pub enum RtmpChunkMessageBody {
    ProtocolControl(ProtocolControlMessage),
    RtmpUserMessage(RtmpMessage),
}

#[repr(u8)]
#[derive(Debug)]
pub enum ChunkMessageType {
    ProtocolControl(ProtocolControlMessageType),
    RtmpUserMessage(RtmpMessageType),
}

impl TryFrom<u8> for ChunkMessageType {
    type Error = ChunkMessageError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if let Ok(v) = ProtocolControlMessageType::try_from(value) {
            return Ok(ChunkMessageType::ProtocolControl(v));
        }

        if let Ok(v) = RtmpMessageType::try_from(value) {
            return Ok(ChunkMessageType::RtmpUserMessage(v));
        }

        Err(ChunkMessageError::UnknownMessageType(value))
    }
}
