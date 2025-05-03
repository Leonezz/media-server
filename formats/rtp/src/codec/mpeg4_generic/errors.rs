use std::io;

use thiserror::Error;

use crate::errors::RtpError;

#[derive(Debug, Error)]
pub enum RtpMpeg4Error {
    #[error("io error: {0}")]
    IoError(#[from] io::Error),
    #[error("invalid stream type: {0}")]
    InvalidStreamType(u8),
    #[error("invalid mode: {0}")]
    InvalidMode(String),
    #[error("parse from fmtp failed: {0}")]
    ParseFromFmtpFailed(String),
    #[error("Syntax error: {0}")]
    SyntaxError(String),
    #[error("Auxiliary data should be empty")]
    AuxiliaryDataEmpty,
    #[error("Access Unit is empty")]
    AccessUnitEmpty,
    #[error("Access Unit Fragment overread, expected {0} bytes, read {1} bytes instead")]
    AccessUnitOverread(usize, usize),
    #[error("Au header count {0} and au count {1} mismatch")]
    AuHeaderCountMissmatch(usize, usize),
    #[error("Rtp Error: {0}")]
    RtpError(#[from] RtpError),
    #[error("Packetize to RTP failed: {0}")]
    PacketizeToRtpFailed(String),
    #[error("Unexpected fragment packet: {0}")]
    UnexpectedFragmentPacket(String),
}

pub type RtpMpeg4Result<T> = Result<T, RtpMpeg4Error>;
