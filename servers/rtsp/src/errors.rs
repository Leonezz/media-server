use rtp_formats::codec::h264::paramters::errors::H264SDPError;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum RtspServerError {
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("rtsp message error: {0}")]
    RtspMessageError(#[from] rtsp_formats::errors::RtspMessageError),
    #[error("sdp error: {0}")]
    SdpError(#[from] sdp_formats::errors::SDPError),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("invalid media description: {0}")]
    InvalidMediaDescription(String),
    #[error("invalid transport: {0}")]
    InvalidTransport(String),
    #[error("unknown encoding name: {0}")]
    InvalidEncodingName(String),
    #[error("invalid H264 SDP Parameters: {0}")]
    InvalidH264SDPParameters(#[from] H264SDPError),
    #[error("invalid param for rtp unpacker: {0}")]
    InvalidParamForRtpUnpacker(String),
    #[error("Gracefully exit")]
    GracefulExit,
}

pub type RtspServerResult<T> = Result<T, RtspServerError>;
