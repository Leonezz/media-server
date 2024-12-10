use std::io;

use rtmp_formats::{chunk::errors::ChunkMessageError, handshake::errors::HandshakeError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RtmpPublishServerError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("handshake failed: {0:?}")]
    HandshakeFailed(#[from] HandshakeError),
    #[error("chunk message read failed: {0:?}")]
    ChunkMessageReadFailed(#[from] ChunkMessageError),
}

pub type RtmpPublishServerResult<T> = Result<T, RtmpPublishServerError>;
