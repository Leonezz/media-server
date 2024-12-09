use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RtmpPublishError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}
