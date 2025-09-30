use stun_formats::message::STUNMessage;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum STUNSessionError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("channel error: {0}")]
    ChannelError(String),
    #[error("stun message error: {0}")]
    MessageError(#[from] stun_formats::errors::STUNMessageError),
    #[error("unknown transaction: {0:?}")]
    UnknownTransaction(STUNMessage),
}

pub type STUNSessionResult<T> = Result<T, STUNSessionError>;
