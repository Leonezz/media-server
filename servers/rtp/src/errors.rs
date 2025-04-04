use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RtpSessionError {
    #[error("io error: {0}")]
    IoError(#[from] io::Error),
    #[error("RTP format error: {0}")]
    RtpFormatError(#[from] rtp_formats::errors::RtpError),
    #[error("RTP packet channel disconnected")]
    RtpPacketChannelDisconnected,
    #[error("RTCP packet channel disconnected")]
    RtcpPacketChannelDisconnected,
    #[error("send rtp packet to channel failed: {0}")]
    SendRtpPacketToChannelFailed(String),
    #[error("send rtcp packet to channel failed: {0}")]
    SendRtcpPacketToChannelFailed(String),
    #[error("gracefully exit")]
    GracefulExit,
}

pub type RtpSessionResult<T> = Result<T, RtpSessionError>;
