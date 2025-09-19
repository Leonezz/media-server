use rtp_formats::{
    codec::{h264::paramters::errors::H264SDPError, mpeg4_generic::errors::RtpMpeg4Error},
    errors::RtpError,
};
use thiserror::Error;
#[derive(Debug, Error)]
pub enum RtspServerError {
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("rtsp message error: {0}")]
    RtspMessageError(#[from] rtsp_formats::errors::RtspMessageError),
    #[error("parse stream properities failed: {0}")]
    ParseStreamProperitiesFailed(
        #[from] server_utils::stream_properities::errors::StreamPropertiesError,
    ),
    #[error("stream center error: {0}")]
    StreamCenterError(#[from] stream_center::errors::StreamCenterError),
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
    #[error("invalid mpeg4-generic SDP Parameters: {0}")]
    InvalidMpeg4GenericSDPParameters(#[from] RtpMpeg4Error),
    #[error("invalid param for rtp unpacker: {0}")]
    InvalidParamForRtpUnpacker(String),
    #[error("no rtp unpacket is set: {0}")]
    NoRtpUnpacker(String),
    #[error("unable to play video stream: {0}")]
    UnableToPlayVideo(String),
    #[error("unable to play audio stream: {0}")]
    UnableToPlayAudio(String),
    #[error("codec parameters error: {0}")]
    CodecParametersError(String),
    #[error("rtp packetize failed: {0}")]
    RtpPacketizeFailed(#[from] RtpError),
    #[error("Gracefully exit")]
    GracefulExit,
}

pub type RtspServerResult<T> = Result<T, RtspServerError>;
