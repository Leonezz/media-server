use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AacError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("wrong sync word: {0}")]
    WrongSyncWord(u16),
}

pub type AacResult<T> = Result<T, AacError>;
