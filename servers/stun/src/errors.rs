use rootcause::Report;
use stun_formats::{builder::MessageBuilder, message::Message};
use thiserror::Error;
#[derive(Debug, Error)]
pub enum StunSessionError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("channel error: {0}")]
    ChannelError(String),
    #[error("stun message error: {0}")]
    MessageError(#[from] stun_formats::errors::StunMessageError),
    #[error("unknown transaction: {0:?}")]
    UnknownTransaction(Message),
}

pub type StunSessionResult<T> = Result<T, Report>;

impl StunSessionError {
    pub fn try_prepare_error_response(
        &self,
        message_builder: &mut MessageBuilder,
    ) -> Result<bool, Report> {
        match self {
            Self::Io(..) | Self::ChannelError(..) | Self::UnknownTransaction(..) => Ok(false),
            Self::MessageError(stun) => stun.try_prepare_error_response(message_builder),
        }
    }
}
