use std::io;

use flv_formats::errors::FLVError;
use stream_center::{errors::StreamCenterError, events::StreamCenterEvent};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpFlvSessionError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("stream event channel send failed: {0:?}")]
    StreamEventSendFailed(Option<StreamCenterEvent>),
    #[error("stream center process event failed: {0:?}")]
    StreamCenterError(#[from] StreamCenterError),
    #[error("process flv tag bytes failed: {0:?}")]
    FlvError(#[from] FLVError),
}

pub type HttpFlvSessionResult<T> = Result<T, HttpFlvSessionError>;
