use std::{io, string};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RtpError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("unknown rtcp payload type: {0}")]
    UnknownRtcpPayloadType(u8),
    #[error("wrong payload type: {0}")]
    WrongPayloadType(String),
    #[error("unknown sdes type: {0}")]
    UnknownSdesType(u8),
    #[error("invalid utf8 data: {0}")]
    InvalidUtf8(#[from] string::FromUtf8Error),
    #[error("invalid compound packet: empty")]
    EmptyCompoundPacket,
    #[error("invalid compound packet: the first rtcp packet must be sr or rr")]
    BadFirstPacket,
    #[error("invalid compound packet: missing cname")]
    MissingCname,
    #[error("invalid compound packet: cmake should be at front")]
    BadCnamePosition,
    #[error("rtp payload is empty")]
    EmptyPayload,
    #[error("Bad padding size: {0}")]
    BadPaddingSize(usize),
}

pub type RtpResult<T> = Result<T, RtpError>;
