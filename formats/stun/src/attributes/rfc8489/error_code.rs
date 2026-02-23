use std::{
    fmt::{self, Debug},
    io::{Read, Write},
};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use utils::traits::dynamic_sized_packet::DynamicSizedPacket;

use crate::{
    MessageChecker,
    attributes::{
        AttributeExtDynamic, AttributeExtStatic, AttributeFactory, STUN_ATTRIBUTE_PADDING_SIZE,
        check_attr_match, get_after_padding_size, rfc8489,
    },
    define_attribute,
    error_codes::{
        ErrorCodeExtDynamic, ErrorCodeExtStatic, from_code, from_code_with_reason,
        rfc8489::{TRY_ALTERNATE, UNAUTHENTICATED, UNKNOWN_ATTRIBUTE},
    },
    errors::{StunMessageError, StunMessageResult},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ErrorCode(u16);
impl Debug for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error code: {}", self.0)
    }
}

impl ErrorCode {
    pub fn get_class(&self) -> u8 {
        (self.0 / 100) as u8
    }

    pub fn get_number(&self) -> u8 {
        (self.0 % 100) as u8
    }

    pub fn code(self) -> u16 {
        self.into()
    }
}

impl From<ErrorCode> for u16 {
    fn from(value: ErrorCode) -> Self {
        value.0
    }
}

impl From<u16> for ErrorCode {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |          Reserved, should be 0          |Class|     Number    |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// | Reason Phrase (variable)                                     ..
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
pub struct ErrorCodeAttribute(Box<dyn ErrorCodeExtDynamic>);

impl Clone for ErrorCodeAttribute {
    fn clone(&self) -> Self {
        Self(crate::error_codes::from_code_with_reason(self.0.code(), self.0.reason()).unwrap())
    }
}

impl ErrorCodeAttribute {
    pub fn new(error_code: ErrorCode) -> StunMessageResult<Self> {
        let err = from_code(error_code.0);
        if err.is_none() {
            return Err(crate::errors::StunMessageError::UnknownErrorCode(
                error_code,
            ));
        }
        Ok(Self(err.unwrap()))
    }

    pub fn new_concrete<E: ErrorCodeExtDynamic + 'static>(error_code: E) -> Self {
        Self(Box::new(error_code))
    }

    pub fn get<T: ErrorCodeExtStatic>(&self) -> Option<T> {
        if self.error_code().0 == T::STATIC_CODE {
            return Some(T::default());
        }
        None
    }

    pub fn new_with_reason<S: Into<String>>(
        error_code: ErrorCode,
        reason_phrase: S,
    ) -> StunMessageResult<Self> {
        let reason = reason_phrase.into();
        if reason.len() > ERROR_CODE_REASON_PHRASE_MAX_LEN {
            return Err(crate::errors::StunMessageError::SyntaxError(format!(
                "length of reason phrase of {:?}: {} exceeds max length: {}",
                Self::STATIC_ATTR_TYPE,
                reason.len(),
                ERROR_CODE_REASON_PHRASE_MAX_LEN
            )));
        }

        let err = from_code_with_reason(error_code.0, &reason);
        if err.is_none() {
            return Err(crate::errors::StunMessageError::UnknownErrorCode(
                error_code,
            ));
        }
        Ok(Self(err.unwrap()))
    }

    pub fn error_code(&self) -> ErrorCode {
        self.0.code().into()
    }

    pub fn reason(&self) -> &str {
        self.0.reason()
    }

    pub fn name(&self) -> &str {
        self.0.name()
    }
}

impl fmt::Debug for ErrorCodeAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AttrType: {:?}, {:?}", Self::STATIC_ATTR_TYPE, self.0)
    }
}

// The reason phrase MUST be a UTF-8-encoded [RFC3629] sequence
// of fewer than 128 characters (which can be as long as
// 509 bytes when encoding them or 763 bytes when decoding them).
pub const ERROR_CODE_REASON_PHRASE_MAX_LEN: usize = 763;

impl DynamicSizedPacket for ErrorCodeAttribute {
    fn get_packet_bytes_count(&self) -> usize {
        get_after_padding_size(4 + self.reason().len(), STUN_ATTRIBUTE_PADDING_SIZE)
    }
}

define_attribute!(0x0009, ErrorCodeAttribute, "ERROR_CODE");

impl MessageChecker for ErrorCodeAttribute {
    fn allowed_in(&self, message_class: crate::header::MessageClass) -> bool {
        matches!(message_class, crate::header::MessageClass::ErrorResponse)
    }
    fn check_error_response(&self, message: &crate::message::Message) -> StunMessageResult<()> {
        let class = message.message_class();
        assert_eq!(class, crate::header::MessageClass::ErrorResponse);
        let error_code = message.get_attribute_ext::<ErrorCodeAttribute>().unwrap();
        match error_code.error_code().code() {
            UNKNOWN_ATTRIBUTE::STATIC_CODE => {
                message.require_ext::<rfc8489::UnknownAttributesAttribute>()?;
            }
            TRY_ALTERNATE::STATIC_CODE => {
                message.require_ext::<rfc8489::AlternateServerAttribute>()?
            }
            UNAUTHENTICATED::STATIC_CODE => {
                if message.has_authentication() {
                    return Err(StunMessageError::InvalidMessage(format!(
                        "no authentication attributes should be inside a {:?} message",
                        error_code
                    )));
                }
            }
            v if v >= 300 && v <= 699 => {}
            v => {
                return Err(StunMessageError::InvalidMessage(format!(
                    "unknown error code: {}",
                    v
                )));
            }
        }
        Ok(())
    }
}

impl AttributeFactory for ErrorCodeAttribute {
    fn from_raw_attr(
        raw_attr: crate::attributes::RawAttribute,
        _transaction_id: &crate::header::TransactionId,
    ) -> Result<Self, StunMessageError> {
        check_attr_match(raw_attr.attr_type, Self::STATIC_ATTR_TYPE)?;
        let mut bytes = raw_attr.value.as_slice();
        let class = (bytes.read_u24::<BigEndian>()? & 0b111) as u16;
        let number = bytes.read_u8()? as u16;
        let error_code = class * 100 + number;
        if bytes.len() > ERROR_CODE_REASON_PHRASE_MAX_LEN {
            return Err(crate::errors::StunMessageError::SyntaxError(format!(
                "length of reason phrase of {:?}: {} exceeds max length: {}",
                Self::STATIC_ATTR_TYPE,
                bytes.len(),
                ERROR_CODE_REASON_PHRASE_MAX_LEN
            )));
        }
        let mut reason_phrase = String::new();
        bytes.read_to_string(&mut reason_phrase)?;
        Self::new_with_reason(error_code.into(), reason_phrase)
    }
    fn into_raw_attr(
        self,
        _transaction_id: &crate::header::TransactionId,
    ) -> crate::attributes::RawAttribute {
        assert!(self.reason().len() < ERROR_CODE_REASON_PHRASE_MAX_LEN);
        let mut value = Vec::with_capacity(self.get_packet_bytes_count());
        value
            .write_u24::<BigEndian>(self.error_code().get_class() as u32)
            .unwrap();
        value.write_u8(self.error_code().get_number()).unwrap();
        value.write_all(self.reason().as_bytes()).unwrap();

        crate::attributes::RawAttribute::new(self.get_type(), value)
    }
}
