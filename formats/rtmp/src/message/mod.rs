use std::io;

use tokio_util::bytes::BytesMut;

use crate::{
    chunk::errors::{ChunkMessageError, ChunkMessageResult},
    commands::{RtmpC2SCommands, RtmpS2CCommands},
    user_control::UserControlEvent,
};

///! difference between rtmp message and rtmp chunk stream message:
/// https://stackoverflow.com/questions/59709461/difference-between-chunk-message-header-and-message-header-in-rtmp
/// https://www.youtube.com/watch?v=AoRepm5ks80&t=1279s
pub mod consts;
pub mod errors;
pub mod reader;
pub mod writer;

///! @see: 6.1.1. Message Header
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// | Message Type  |                 Payload length                |
/// |   (1 byte)    |                    (3 bytes)                  |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                            Timestamp                          |
/// |                            (4 bytes)                          |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                   Stream ID                   |
/// |                   (3 bytes)                   |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///
#[derive(Debug)]
pub struct RtmpMessageHeader {
    message_type: RtmpMessageType, // 1 byte, should be the same as message_type_id in chunk message header ?
    payload_length: u32,           // 3 bytes
    timestamp: u32,                // 4 bytes
    stream_id: u32,                // 3 bytes
}

#[derive(Debug)]
pub struct RtmpMessage {
    header: RtmpMessageHeader,
    message: RtmpUserMessageBody,
}

#[derive(Debug)]
pub enum RtmpUserMessageBody {
    UserControl(UserControlEvent),
    C2SCommand(RtmpC2SCommands),
    S2Command(RtmpS2CCommands),
    MetaData(amf::Value),
    SharedObject(/*TODO */),
    Audio { payload: BytesMut },
    Video { payload: BytesMut },
    Aggregate { payload: BytesMut },
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum RtmpMessageType {
    UserControl = 4,
    AMF3Command = 17,
    AMF0Command = 20,
    AMF3Data = 15,
    AMF0Data = 18,
    AMF3SharedObject = 16,
    AMF0SharedObject = 19,
    Audio = 8,
    Video = 9,
    Aggregate = 22,
}

impl Into<u8> for RtmpMessageType {
    fn into(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for RtmpMessageType {
    type Error = ChunkMessageError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            4 => Ok(RtmpMessageType::UserControl),
            17 => Ok(RtmpMessageType::AMF3Command),
            20 => Ok(RtmpMessageType::AMF0Command),
            15 => Ok(RtmpMessageType::AMF3Data),
            18 => Ok(RtmpMessageType::AMF0Data),
            16 => Ok(RtmpMessageType::AMF3SharedObject),
            19 => Ok(RtmpMessageType::AMF0SharedObject),
            8 => Ok(RtmpMessageType::Audio),
            9 => Ok(RtmpMessageType::Video),
            22 => Ok(RtmpMessageType::Aggregate),
            _ => Err(ChunkMessageError::UnknownMessageType(value)),
        }
    }
}

impl RtmpMessage {
    pub fn read_c2s_from<R>(inner: R, version: amf::Version) -> ChunkMessageResult<RtmpMessage>
    where
        R: io::Read,
    {
        reader::Reader::new(inner).read_c2s(version)
    }

    pub fn read_s2c_from<R>(inner: R, version: amf::Version) -> ChunkMessageResult<RtmpMessage>
    where
        R: io::Read,
    {
        todo!()
    }

    pub fn write_c2s_to<W>(&self, inner: W, version: amf::Version) -> ChunkMessageResult<()>
    where
        W: io::Write,
    {
        writer::Writer::new(inner).write(self)
    }
}