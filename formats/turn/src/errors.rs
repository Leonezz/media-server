use stun_formats::builder::MessageBuilder;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum TurnMessageError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("stun message error: {0}")]
    Stun(#[from] stun_formats::errors::StunMessageError),
    #[error("not a channel data message, first byte: {0}")]
    NotChannelData(u8),
    #[error("unknown turn first byte: {0}")]
    UnknownFirstByte(u8),
}

pub type TurnMessageResult<T> = Result<T, TurnMessageError>;

impl TurnMessageError {
    pub fn try_prepare_error_response(
        self,
        message_builder: &mut MessageBuilder,
    ) -> TurnMessageResult<()> {
        match self {
            Self::Io(_) | Self::NotChannelData(_) | Self::UnknownFirstByte(_) => Err(self),
            Self::Stun(stun) => stun
                .try_prepare_error_response(message_builder)
                .map_err(TurnMessageError::Stun),
        }
    }
}
