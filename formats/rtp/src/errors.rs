use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RtpError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}

pub type RtpResult<T> = Result<T, RtpError>;
