use std::string::FromUtf8Error;

use thiserror::Error;

use crate::{
    attributes::rfc8489::{ErrorCodeAttribute, UnknownAttributesAttribute},
    builder::MessageBuilder,
    error_codes::rfc8489::{BAD_REQUEST, UNKNOWN_ATTRIBUTE},
};

#[derive(Debug, Error)]
pub enum StunMessageError {
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Syntax error: {0}")]
    SyntaxError(String),
    #[error("invalid utf8 string")]
    InvalidUtf8String(#[from] FromUtf8Error),
    #[error("unknown error code: {0:?}")]
    UnknownErrorCode(crate::attributes::rfc8489::error_code::ErrorCode),
    #[error("invalid message: {0}")]
    InvalidMessage(String),
    #[error("unknown attribute: {0:?}")]
    UnknownAttributes(Vec<u16>),
    #[error("unknown method: {0}")]
    UnknownMethod(u16),
    #[error("builder error: {0}")]
    BuilderError(String),
}

pub type StunMessageResult<T> = Result<T, StunMessageError>;

impl StunMessageError {
    pub fn try_prepare_error_response(
        self,
        message_builder: &mut MessageBuilder,
    ) -> StunMessageResult<()> {
        match self {
            Self::IoError(..) => Err(self),
            Self::BuilderError(_) => Err(self),
            Self::SyntaxError(_)
            | Self::InvalidUtf8String(_)
            | Self::InvalidMessage(_)
            | Self::UnknownMethod(..)
            | Self::UnknownErrorCode(_) => {
                message_builder
                    .error_mut()
                    .attribute_mut(ErrorCodeAttribute::new_concrete(
                        BAD_REQUEST::new_with_reason(self.to_string()),
                    ))?;
                Ok(())
            }
            Self::UnknownAttributes(attrs) => {
                message_builder
                    .error_mut()
                    .attribute_mut(ErrorCodeAttribute::new_concrete(UNKNOWN_ATTRIBUTE::new()))?
                    .attribute_mut(UnknownAttributesAttribute { attributes: attrs })?;
                Ok(())
            }
        }
    }
}
