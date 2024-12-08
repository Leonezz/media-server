use errors::errors::RtmpMessageError;

///! @see: 7.1.7. User Control Message Events
pub mod consts;
mod errors;
pub mod reader;
pub mod writer;

#[derive(Debug)]
pub enum UserControlEvent {
    StreamBegin {
        stream_id: u32,
    },
    StreamEOF {
        stream_id: u32,
    },
    StreamDry {
        stream_id: u32,
    },
    SetBufferLength {
        stream_id: u32,     // first 4 bytes in the event payload
        buffer_length: u32, // buffer length in millis
    },
    StreamIdsRecorded {
        stream_id: u32,
    },
    PingRequest {
        timestamp: u32,
    },
    PingResponse {
        timestamp: u32,
    },
}

#[repr(u16)]
#[derive(Debug)]
pub enum UserControlEventType {
    StreamBegin = 0,
    StreamEOF = 1,
    StreamDry = 2,
    SetBufferLength = 3,
    StreamIdsRecorded = 4,
    PingRequest = 6,
    PingResponse = 7,
}

impl Into<u16> for UserControlEventType {
    fn into(self) -> u16 {
        self as u16
    }
}

impl TryFrom<u16> for UserControlEventType {
    type Error = RtmpMessageError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(UserControlEventType::StreamBegin),
            1 => Ok(UserControlEventType::StreamEOF),
            2 => Ok(UserControlEventType::StreamDry),
            3 => Ok(UserControlEventType::SetBufferLength),
            4 => Ok(UserControlEventType::StreamIdsRecorded),
            6 => Ok(UserControlEventType::PingRequest),
            7 => Ok(UserControlEventType::PingResponse),
            _ => Err(RtmpMessageError::UnknownEventType(value)),
        }
    }
}
