use stun_formats::message::Message;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum STUNSessionError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("channel error: {0}")]
    ChannelError(String),
    #[error("stun message error: {0}")]
    MessageError(#[from] stun_formats::errors::StunMessageError),
    #[error("unknown transaction: {0:?}")]
    UnknownTransaction(Message),
}

pub type STUNSessionResult<T> = Result<T, STUNSessionError>;
