use std::{io, string};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RtpError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid h264 codec: {0}")]
    InvalidH264Codec(String),
    #[error("unknown rtcp payload type: {0}")]
    UnknownRtcpPayloadType(u8),
    #[error("wrong payload type: {0}")]
    WrongPayloadType(String),
    #[error("unknown sdes type: {0}")]
    UnknownSdesType(u8),
    #[error("sdes value too large, exceeds u8 length: {0}")]
    SDESValueTooLarge(String),
    #[error("bye reason too large, exceeds u8 length: {0}")]
    ByeReasonTooLarge(String),
    #[error("sdes packet has too many chunks, exceeds 31")]
    SDESTooManyChunks,
    #[error("invalid utf8 data: {0}")]
    InvalidUtf8(#[from] string::FromUtf8Error),
    #[error("invalid compound packet: empty")]
    EmptyRtcpCompoundPacket,
    #[error("invalid compound packet: the first rtcp packet must be sr or rr")]
    BadFirstPacketInRtcpCompound,
    #[error("invalid compound packet: missing cname")]
    MissingCnameInRtcpCompound,
    #[error("invalid compound packet: cmake should be at front")]
    BadCnamePositionInRtcpCompound,
    #[error("rtp payload is empty")]
    EmptyPayload,
    #[error("Bad padding size: {0}")]
    BadPaddingSize(usize),
    #[error("too many csrc for a rtp header, exceeds 31")]
    TooManyCSRC,
    #[error("too many report blocks in a report packet, exceeds 31")]
    TooManyReportBlocks,

    #[error("MTU is too small: {0}")]
    MTUTooSmall(usize),
    #[error("payload too large, exceeds u16 length: {0}")]
    PayloadTooLarge(usize),
}

pub type RtpResult<T> = Result<T, RtpError>;
