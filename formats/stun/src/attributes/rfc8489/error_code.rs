use std::{
    fmt,
    io::{Read, Write},
};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use utils::traits::dynamic_sized_packet::DynamicSizedPacket;

use crate::{
    attribute::STUNAttributeExt,
    attributes::{STUN_ATTRIBUTE_PADDING_SIZE, check_attr_match, get_after_padding_size},
    errors::{STUNMessageError, STUNMessageResult},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ErrorCode(u16);

impl ErrorCode {
    pub fn get_class(&self) -> u8 {
        (self.0 / 100) as u8
    }

    pub fn get_number(&self) -> u8 {
        (self.0 % 100) as u8
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

impl fmt::Debug for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (*self).try_into() {
            Err(_) => write!(f, "{}", self.0),
            Ok(str) => f.write_str(str),
        }
    }
}

///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |          Reserved, should be 0          |Class|     Number    |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// | Reason Phrase (variable)                                     ..
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Clone)]
pub struct ErrorCodeAttribute {
    error_code: ErrorCode,
    reason_phrase: String,
}

impl ErrorCodeAttribute {
    pub fn new(error_code: ErrorCode) -> STUNMessageResult<Self> {
        let reason: &str = error_code.try_into().unwrap_or("unknown");
        Self::new_with_reason(error_code, reason)
    }

    pub fn new_with_reason(error_code: ErrorCode, reason_phrase: &str) -> STUNMessageResult<Self> {
        if reason_phrase.len() > ERROR_CODE_REASON_PHRASE_MAX_LEN {
            return Err(crate::errors::STUNMessageError::SyntaxError(format!(
                "length of reason phrase of {:?}: {} exceeds max length: {}",
                crate::attribute::AttrType::ErrorCode,
                reason_phrase.len(),
                ERROR_CODE_REASON_PHRASE_MAX_LEN
            )));
        }

        Ok(Self {
            error_code,
            reason_phrase: reason_phrase.to_owned(),
        })
    }

    pub fn error_code(&self) -> ErrorCode {
        self.error_code
    }

    pub fn reason_phrase(&self) -> &String {
        &self.reason_phrase
    }
}

impl fmt::Debug for ErrorCodeAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AttrType: {:?}, code: {:?}, reason: {}",
            self.get_type(),
            self.error_code,
            self.reason_phrase
        )
    }
}

// The reason phrase MUST be a UTF-8-encoded [RFC3629] sequence
// of fewer than 128 characters (which can be as long as
// 509 bytes when encoding them or 763 bytes when decoding them).
pub const ERROR_CODE_REASON_PHRASE_MAX_LEN: usize = 763;

impl DynamicSizedPacket for ErrorCodeAttribute {
    fn get_packet_bytes_count(&self) -> usize {
        get_after_padding_size(4 + self.reason_phrase.len(), STUN_ATTRIBUTE_PADDING_SIZE)
    }
}

impl STUNAttributeExt for ErrorCodeAttribute {
    fn get_type(&self) -> crate::attribute::AttrType {
        crate::attribute::AttrType::ErrorCode
    }

    fn from_raw_attr(
        raw_attr: crate::attribute::STUNRawAttribute,
        _transaction_id: &[u8; crate::header::TRANSACTION_ID_LEN],
    ) -> Result<Self, crate::errors::STUNMessageError> {
        check_attr_match(raw_attr.attr_type, crate::attribute::AttrType::ErrorCode)?;
        let mut bytes = raw_attr.value.as_slice();
        let class = (bytes.read_u24::<BigEndian>()? & 0b111) as u16;
        let number = bytes.read_u8()? as u16;
        let error_code = class * 100 + number;
        if bytes.len() > ERROR_CODE_REASON_PHRASE_MAX_LEN {
            return Err(crate::errors::STUNMessageError::SyntaxError(format!(
                "length of reason phrase of {:?}: {} exceeds max length: {}",
                crate::attribute::AttrType::ErrorCode,
                bytes.len(),
                ERROR_CODE_REASON_PHRASE_MAX_LEN
            )));
        }
        let mut reason_phrase = String::new();
        bytes.read_to_string(&mut reason_phrase)?;
        Ok(Self {
            error_code: error_code.into(),
            reason_phrase,
        })
    }

    fn into_raw_attr(
        self,
        _transaction_id: &[u8; crate::header::TRANSACTION_ID_LEN],
    ) -> crate::attribute::STUNRawAttribute {
        assert!(self.reason_phrase.len() < ERROR_CODE_REASON_PHRASE_MAX_LEN);
        let mut value = Vec::with_capacity(self.get_packet_bytes_count());
        value
            .write_u24::<BigEndian>(self.error_code.get_class() as u32)
            .unwrap();
        value.write_u8(self.error_code.get_number()).unwrap();
        value.write_all(self.reason_phrase.as_bytes()).unwrap();

        crate::attribute::STUNRawAttribute::new(self.get_type(), value)
    }
}

/// 300 Try Alternate:
/// The client should contact an alternate server for this request.
/// This error response MUST only be sent if the request included either
/// a USERNAME or USERHASH attribute and a valid MESSAGE-INTEGRITY or
/// MESSAGE-INTEGRITY-SHA256 attribute; otherwise,
/// it MUST NOT be sent and error code 400 (Bad Request) is suggested.
/// This error response MUST be protected with the MESSAGE-INTEGRITY or
/// MESSAGE-INTEGRITY-SHA256 attribute,
/// and receivers MUST validate the MESSAGE-INTEGRITY or
/// MESSAGEINTEGRITY-SHA256 of this response before redirecting themselves to an alternate server.
pub const ERROR_CODE_TRY_ALTERNATE: ErrorCode = ErrorCode(300);
pub const ERROR_REASON_TRY_ALTERNATE: &str = "Try Alternate";

/// 400 Bad Request:
/// The request was malformed.
/// The client SHOULD NOT retry the request without modification
/// from the previous attempt.
/// The server may not be able to generate a valid MESSAGE-INTEGRITY or
/// MESSAGE-INTEGRITY-SHA256 for this error,
/// so the client MUST NOT expect a valid MESSAGE-INTEGRITY or
/// MESSAGEINTEGRITY-SHA256 attribute on this response.
pub const ERROR_CODE_BAD_REQUEST: ErrorCode = ErrorCode(400);
pub const ERROR_REASON_BAD_REQUEST: &str = "Bad Request";

/// 401 Unauthenticated:
/// The request did not contain the correct credentials to proceed.
/// The client should retry the request with proper credentials.
pub const ERROR_CODE_UNAUTHENTICATED: ErrorCode = ErrorCode(401);
pub const ERROR_REASON_UNAUTHENTICATED: &str = "Unauthenticated";

/// 420 Unknown Attribute:
/// The server received a STUN packet containing a comprehension-required attribute
/// that it did not understand.
/// The server MUST put this unknown attribute in the UNKNOWNATTRIBUTE attribute of its error response.
pub const ERROR_CODE_UNKNOWN_ATTRIBUTE: ErrorCode = ErrorCode(420);
pub const ERROR_REASON_UNKNOWN_ATTRIBUTE: &str = "Unknown Attribute";

/// 438 Stale Nonce:
/// The NONCE used by the client was no longer valid.
/// The client should retry, using the NONCE provided in the response.
pub const ERROR_CODE_STALE_NONCE: ErrorCode = ErrorCode(438);
pub const ERROR_REASON_STALE_NONCE: &str = "Stale Nonce";

/// 500 Server Error:
/// The server has suffered a temporary error. The client should try again.
pub const ERROR_CODE_SERVER_ERROR: ErrorCode = ErrorCode(500);
pub const ERROR_REASON_SERVER_ERROR: &str = "Server Error";

impl TryFrom<ErrorCode> for &'static str {
    type Error = STUNMessageError;
    fn try_from(value: ErrorCode) -> Result<Self, Self::Error> {
        let reason = match value {
            ERROR_CODE_TRY_ALTERNATE => ERROR_REASON_TRY_ALTERNATE,
            ERROR_CODE_BAD_REQUEST => ERROR_REASON_BAD_REQUEST,
            ERROR_CODE_UNAUTHENTICATED => ERROR_REASON_UNAUTHENTICATED,
            ERROR_CODE_UNKNOWN_ATTRIBUTE => ERROR_REASON_UNKNOWN_ATTRIBUTE,
            ERROR_CODE_STALE_NONCE => ERROR_REASON_STALE_NONCE,
            ERROR_CODE_SERVER_ERROR => ERROR_REASON_SERVER_ERROR,
            _ => return Err(STUNMessageError::UnknownErrorCode(value)),
        };
        Ok(reason)
    }
}
