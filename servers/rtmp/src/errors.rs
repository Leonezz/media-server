use std::{backtrace::Backtrace, io};

use rtmp_formats::{chunk::errors::ChunkMessageError, handshake::errors::HandshakeError};
use stream_center::errors::StreamCenterError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RtmpServerError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("handshake failed: {0:?}")]
    HandshakeFailed(#[from] HandshakeError),
    #[error("chunk message read failed: {0:?}")]
    ChunkMessageReadFailed(#[from] ChunkMessageError),
    #[error("flv tag demux error: {0}")]
    FlvDemuxError(#[from] flv_formats::errors::FLVError),
    #[error("stream center operation error")]
    StreamCenterError(#[from] StreamCenterError),
    #[error("channel send data failed, {backtrace}")]
    ChannelSendFailed { backtrace: Backtrace },
    #[error("invalid stream param: {0}")]
    InvalidStreamParam(String),
    #[error("stream is gone")]
    StreamIsGone,
}

pub type RtmpServerResult<T> = Result<T, RtmpServerError>;
