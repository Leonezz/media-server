use std::io;

use h264_codec::errors::H264CodecError;
use thiserror::Error;

use crate::errors::RtpError;

use super::paramters::packetization_mode::PacketizationMode;
#[derive(Debug, Error)]
pub enum RtpH264Error {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("h264 codec error: {0:?}")]
    H264CodecError(#[from] H264CodecError),
    #[error("RTP error: {0:?}")]
    RtpError(#[from] RtpError),
    #[error("invalid mtu: {0}")]
    InvalidMTU(usize),
    #[error("Invalid packetization mode: {0}")]
    InvalidPacketizationMode(String),
    #[error("invalid packet type for h264: {0}")]
    InvalidH264PacketType(u8),
    #[error("rtp sequencing h264 nal from fu packets failed: {0}")]
    SequenceFUPacketsFailed(String),
    #[error("unexpected packet type: {0}")]
    UnexpectedPacketType(String),
    #[error("unsupported packetization mode: {0}")]
    UnsupportedPacketizationMode(PacketizationMode),
}

pub type RtpH264Result<T> = Result<T, RtpH264Error>;
