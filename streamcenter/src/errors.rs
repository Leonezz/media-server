use std::{backtrace::Backtrace, io};

use thiserror::Error;

use crate::stream_source::StreamIdentifier;
#[derive(Debug, Error)]
pub enum StreamCenterError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("stream is already publishing {0:?}")]
    DuplicateStream(StreamIdentifier),
    #[error("stream not found: {0:?}")]
    StreamNotFound(StreamIdentifier),
    #[error("channel send failed, {backtrace}")]
    ChannelSendFailed { backtrace: Backtrace },
    #[error("invalid stream type: {0}")]
    InvalidStreamType(String),
    #[error("parse flv tag failed")]
    ParseFLVTagFailed(#[from] flv_formats::errors::FLVError),
    #[error("parse h264 codec elements failed: {0}")]
    H264CodecError(#[from] codec_h264::errors::H264CodecError),
    #[error("parse aac codec elements failed: {0}")]
    AACCodecError(#[from] codec_aac::errors::AACCodecError),
    #[error("remux failed: {0}")]
    RemuxFailed(String),
    #[error("mix queue full: {0} {1}")]
    MixQueueFull(String, usize),
}

pub type StreamCenterResult<T> = Result<T, StreamCenterError>;
