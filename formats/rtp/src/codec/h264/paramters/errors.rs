use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum H264SDPError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid format: {0}")]
    InvalidFormat(String),
    #[error("invalid profile_level_id: {0}")]
    InvalidProfileLevelId(String),
    #[error("invalid max_recv_level: {0}")]
    InvalidMaxRecvLevel(String),
    #[error("invalid packetization mode: {0}")]
    InvalidPacketizationMode(String),
    #[error("invalid sprop-deint-buf-req: {0}")]
    InvalidSpropDeintBufReq(String),
    #[error("invalid sprop-interleaving-depth: {0}")]
    InvalidSpropInterleavingDepth(String),
    #[error("invalid sprop-max-don-diff: {0}")]
    InvalidSpropMaxDonDiff(String),
    #[error("invalid sprop-init-buf-time: {0}")]
    InvalidSpropInitBufTime(String),
    #[error("invalid max-mbps: {0}")]
    InvalidMaxMbps(String),
    #[error("invalid max-smbps: {0}")]
    InvalidMaxSmbps(String),
    #[error("invalid max-fs: {0}")]
    InvalidMaxFs(String),
    #[error("invalid max-cpb: {0}")]
    InvalidMaxCpb(String),
    #[error("invalid max-dpb: {0}")]
    InvalidMaxDpb(String),
    #[error("invalid max-br: {0}")]
    InvalidMaxBr(String),
    #[error("invalid redundant-pic-cap: {0}")]
    InvalidRedundantPicCap(String),
    #[error("invalid deint-buf-cap: {0}")]
    InvalidDeintBufCap(String),
    #[error("invalid max-rcmd-nalu-size: {0}")]
    InvalidMaxRcmdNaluSize(String),
    #[error("invalid sar-understood: {0}")]
    InvalidSarUnderstood(String),
    #[error("invalid sar-supported: {0}")]
    InvalidSarSupported(String),
    #[error("invalid in-band-parameter-sets: {0}")]
    InvalidInBandParameterSets(String),
    #[error("invalid use-level-src-parameter-sets: {0}")]
    InvalidUseLevelSrcParameterSets(String),
    #[error("invalid level-asymmetry-allowed: {0}")]
    InvalidLevelAsymmetryAllowed(String),
    #[error("invalid sprop-parameter-sets: {0}")]
    InvalidSpropParameterSets(String),
    #[error("invalid sprop-level-parameter-sets: {0}")]
    InvalidSpropLevelParameterSets(String),
    #[error("convert from h264 fmtp parameter to avc decoder configuration record failed: {0}")]
    FmptToAvcDecoderConfigurationRecordError(String),
}

pub type H264SDPResult<T> = Result<T, H264SDPError>;
