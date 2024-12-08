use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CommandMessageError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("amf error: {0}")]
    AmfError(#[from] amf::errors::AmfError),
    #[error("unexpected amf type: {0}")]
    UnexpectedAmfType(String),
    #[error("unexpected command name: {0}")]
    UnexpectedCommandName(String),
    #[error("unknown amf version: {0}")]
    UnknownAmfVersion(u8),
}

pub type CommandMessageResult<T> = Result<T, CommandMessageError>;
