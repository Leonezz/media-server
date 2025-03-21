use std::backtrace::Backtrace;

use errors::{ChunkMessageError, ChunkMessageResult};

use crate::{
    message::{RtmpMessageType, RtmpUserMessageBody},
    protocol_control::{ProtocolControlMessage, ProtocolControlMessageType},
    user_control::{UserControlEvent, consts::USER_CONTROL_MESSAGE_TYPE},
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

type Csid = u32;

// @see: 5.3.1.1. Chunk Basic Header
// 1, 2 or 3 bytes
#[derive(Debug, Clone)]
pub struct ChunkBasicHeader {
    header_type: ChunkBasicHeaderType,
    fmt: u8,               // 2 bits
    chunk_stream_id: Csid, // 6 bits / 1 byte / 2 bytes
}

impl ChunkBasicHeader {
    pub fn new(fmt: u8, csid: Csid) -> ChunkMessageResult<Self> {
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

    pub fn get_header_length(&self) -> usize {
        match self.header_type {
            ChunkBasicHeaderType::Type1 => 1,
            ChunkBasicHeaderType::Type2 => 2,
            ChunkBasicHeaderType::Type3 => 3,
        }
    }
}

// @see: 5.3.1.2. Chunk Message Header
// @see: 5.3.1.2.1. Type 0 - for start of a chunk stream, or for timestamp backwards
// 11 bytes total
// the timestamp fields must by less or equal then 0xFFFF, and enables extend timestamp field if timestamp equals to 0xFFFF
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

// @see: 5.3.1.2.2. Type 1 - this chunk takes the same stream ID as the preceding chunk
// 7 bytes
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

// @see: 5.3.1.2.3. Type 2 - for streams with constant-sized messages
// 3 bytes
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                timestamp delta                |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///         Chunk Message Header - Type 2
#[derive(Debug)]
pub struct ChunkMessageHeaderType2 {
    timestamp_delta: u32, // 3 bytes
}

// @see: 5.3.1.2.4. Type 3 - for one message split into multiple chunks
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

impl ChunkMessageHeader {
    pub fn get_header_length(&self) -> usize {
        match self {
            ChunkMessageHeader::Type0(_) => 11,
            ChunkMessageHeader::Type1(_) => 7,
            ChunkMessageHeader::Type2(_) => 3,
            ChunkMessageHeader::Type3(_) => 0,
        }
    }
}

#[derive(Debug, Default)]
pub struct RuntimeStat {
    pub read_time_ns: u128,
    pub process_time_ns: u128,
}

#[derive(Debug)]
pub struct ChunkMessageCommonHeader {
    pub basic_header: ChunkBasicHeader,
    pub timestamp: u32,
    pub message_length: u32,
    pub message_type_id: u8,
    pub message_stream_id: u32,
    pub extended_timestamp_enabled: bool,

    pub runtime_stat: RuntimeStat,
}

// @see: 5.3.1. Chunk Format
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
    UserControl(UserControlEvent),
    RtmpUserMessage(Box<RtmpUserMessageBody>),
}

#[repr(u8)]
#[derive(Debug)]
pub enum ChunkMessageType {
    ProtocolControl(ProtocolControlMessageType),
    UserControl = USER_CONTROL_MESSAGE_TYPE,
    RtmpUserMessage(RtmpMessageType),
}

impl TryFrom<u8> for ChunkMessageType {
    type Error = ChunkMessageError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value == USER_CONTROL_MESSAGE_TYPE {
            return Ok(ChunkMessageType::UserControl);
        }

        if let Ok(v) = ProtocolControlMessageType::try_from(value) {
            return Ok(ChunkMessageType::ProtocolControl(v));
        }

        if let Ok(v) = RtmpMessageType::try_from(value) {
            return Ok(ChunkMessageType::RtmpUserMessage(v));
        }

        Err(ChunkMessageError::UnknownMessageType {
            type_id: value,
            backtrace: Backtrace::capture(),
        })
    }
}
