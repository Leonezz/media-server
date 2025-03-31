use thiserror::Error;

#[derive(Debug, Error)]
pub enum RtpSessionError {
    #[error("RTP format error: {0}")]
    RtpFormatError(#[from] rtp_formats::errors::RtpError),
    #[error("RTP packet channel disconnected")]
    RtpPacketChannelDisconnected,
}

pub type RtpSessionResult<T> = Result<T, RtpSessionError>;
