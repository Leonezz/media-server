use std::{backtrace::Backtrace, io};

use crate::chunk::errors::{ChunkMessageError, ChunkMessageResult};

pub mod consts;
pub mod errors;
pub mod reader;
pub mod writer;

///! @see: 5.4.1. Set Chunk Size (1)
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |0|                     chunk size (31 bits)                    |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///         Payload for the "Set Chunk Size" protocol message
#[derive(Debug)]
pub struct SetChunkSize {
    pub chunk_size: u32, // 31 bits, in [1, 0xFFFFFF]
}

///! @see: 5.4.2. Abort Message (2)
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                   chunk stream id (32 bits)                   |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///         Payload for the "Abort Message" protocol message
#[derive(Debug)]
pub struct AbortMessage {
    pub chunk_stream_id: u32,
}

///! @see: 5.4.3. Acknowledgement (3)
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                   sequence number (4 bytes)                   |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///        Payload for the "Acknowledgement" protocol message
#[derive(Debug)]
pub struct Acknowledgement {
    pub sequence_number: u32,
}

///! @see: 5.4.4. Window Acknowledgement Size (5)
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |             Acknowledgement Window size (4 bytes)             |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///  Payload for the "Window Acknowledgement Size" protocol message
#[derive(Debug)]
pub struct WindowAckSize {
    pub size: u32,
}

#[repr(u8)]
#[derive(Debug)]
pub enum ProtocolControlMessageType {
    SetChunkSize = 1,
    Abort = 2,
    Acknowledgement = 3,
    WindowAckSize = 5,
    SetPeerBandwidth = 6,
}

impl Into<u8> for ProtocolControlMessageType {
    fn into(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for ProtocolControlMessageType {
    type Error = ChunkMessageError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(ProtocolControlMessageType::SetChunkSize),
            2 => Ok(ProtocolControlMessageType::Abort),
            3 => Ok(ProtocolControlMessageType::Acknowledgement),
            5 => Ok(ProtocolControlMessageType::WindowAckSize),
            6 => Ok(ProtocolControlMessageType::SetPeerBandwidth),
            _ => Err(ChunkMessageError::UnknownMessageType {
                type_id: value,
                backtrace: Backtrace::capture(),
            }),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetPeerBandWidthLimitType {
    // The peer SHOULD limit its output bandwidth to the indicated window size.
    Hard = 0,
    // The peer SHOULD limit its output bandwidth to the the window indicated in this message
    // or the limit already in effect, whichever is smaller.
    Soft = 1,
    // If the previous Limit Type was Hard,
    // treat this message as though it was marked Hard, otherwise ignore this message.
    Dynamic = 2,
}

impl TryFrom<u8> for SetPeerBandWidthLimitType {
    type Error = ChunkMessageError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(SetPeerBandWidthLimitType::Hard),
            1 => Ok(SetPeerBandWidthLimitType::Soft),
            2 => Ok(SetPeerBandWidthLimitType::Dynamic),
            _ => Err(ChunkMessageError::InvalidMessage(format!(
                "invalid set peer bandwidth message, the limit type is unknown: {}",
                value
            ))),
        }
    }
}

///! @see: 5.4.5. Set Peer Bandwidth (6)
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                  Acknowledgement Window size                  |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |  Limit Type   |
/// +-+-+-+-+-+-+-+-+
///       Payload for the "Set Peer Bandwidth" protocol message
#[derive(Debug)]
pub struct SetPeerBandwidth {
    pub size: u32,
    pub limit_type: SetPeerBandWidthLimitType,
}

#[derive(Debug)]
pub enum ProtocolControlMessage {
    SetChunkSize(SetChunkSize),
    Abort(AbortMessage),
    Ack(Acknowledgement),
    WindowAckSize(WindowAckSize),
    SetPeerBandwidth(SetPeerBandwidth),
}

impl ProtocolControlMessage {
    pub fn read_from<R>(
        inner: R,
        message_type: ProtocolControlMessageType,
    ) -> ChunkMessageResult<ProtocolControlMessage>
    where
        R: io::Read,
    {
        reader::Reader::new(inner).read(message_type)
    }

    pub fn write_to<W>(&self, inner: W) -> ChunkMessageResult<()>
    where
        W: io::Write,
    {
        writer::Writer::new(inner).write(self)
    }
}
