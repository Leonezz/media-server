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
}

pub type RtspServerResult<T> = Result<T, RtspServerError>;
