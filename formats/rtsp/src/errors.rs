use std::io;

use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum RtspMessageError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("format error: {0}")]
    FormatError(#[from] std::fmt::Error),
    #[error("Unknown method: {0:?}")]
    UnknownMethod(Option<String>),
    #[error("Unknown uri: {0:?}")]
    UnknownUri(Option<Url>),
    #[error("Unknown header: {0:?}")]
    UnknownHeader(Option<String>),
    #[error("Unknown status code: {0:?}")]
    UnknownStatusCode(Option<u16>),
    #[error("Unknown rtsp version: {0:?}")]
    UnknownRtspVersion(Option<String>),
    #[error("Missing Content-Type header for a message with a body")]
    MissingContentType,
    #[error("Missing Content-Length header for a message with a body")]
    MissingContentLength,
    #[error("Invalid message format: {0}")]
    InvalidRtspMessageFormat(String),
    #[error("Invalid Url: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("Invalid interleaved $ sign: {0}")]
    InvalidInterleavedSign(u8),
    #[error("Invalid interleaved data length: {0}")]
    InvalidInterleavedDataLength(usize),
    #[error("Invalid sdp control attribute: {0}")]
    InvalidSdpControlAttribute(String),
    #[error("Invalid sdp time range: {0}")]
    InvalidSdpRangeAttribute(String),
    #[error("Invalid Absolute Time: {0}")]
    InvalidAbsoluteTime(String),
    #[error("Invalid npt: {0}")]
    InvalidNPT(String),
    #[error("Invalid smpte: {0}")]
    InvalidSMPTE(String),
}

pub type RtspMessageResult<T> = Result<T, RtspMessageError>;
