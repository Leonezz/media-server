use thiserror::Error;
#[derive(Debug, Error)]
pub enum RtspServerError {
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("rtsp message error: {0}")]
    RtspMessageError(#[from] rtsp_formats::errors::RTSPMessageError),
}

pub type RtspServerResult<T> = Result<T, RtspServerError>;
