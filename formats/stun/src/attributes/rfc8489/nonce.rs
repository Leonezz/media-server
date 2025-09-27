use std::fmt;

use utils::traits::dynamic_sized_packet::DynamicSizedPacket;

use crate::{
    attribute::STUNAttributeExt,
    attributes::{STUN_ATTRIBUTE_PADDING_SIZE, check_attr_match, get_after_padding_size},
    errors::STUNMessageResult,
};

#[derive(Clone)]
pub struct NonceAttribute {
    value: String,
}

impl NonceAttribute {
    pub fn new(value: &str) -> STUNMessageResult<Self> {
        if value.len() > NONCE_VALUE_MAX_LEN {
            return Err(crate::errors::STUNMessageError::SyntaxError(format!(
                "value length for {:?}: {} exceeds max length: {}",
                crate::attribute::AttrType::Nonce,
                value.len(),
                NONCE_VALUE_MAX_LEN
            )));
        }

        Ok(Self {
            value: value.to_owned(),
        })
    }

    pub fn value(&self) -> &String {
        &self.value
    }
}

impl fmt::Debug for NonceAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AttrType: {:?}, value: {}", self.get_type(), self.value)
    }
}

/// The NONCE attribute MUST be fewer than 128 characters
/// (which can be as long as 509 bytes when encoding them and a long as 763 bytes when decoding them)
pub const NONCE_VALUE_MAX_LEN: usize = 763;

impl DynamicSizedPacket for NonceAttribute {
    fn get_packet_bytes_count(&self) -> usize {
        get_after_padding_size(self.value.len(), STUN_ATTRIBUTE_PADDING_SIZE)
    }
}

impl STUNAttributeExt for NonceAttribute {
    fn get_type(&self) -> crate::attribute::AttrType {
        crate::attribute::AttrType::Nonce
    }

    fn from_raw_attr(
        raw_attr: crate::attribute::STUNRawAttribute,
        _transaction_id: &[u8; crate::header::TRANSACTION_ID_LEN],
    ) -> Result<Self, crate::errors::STUNMessageError> {
        check_attr_match(raw_attr.attr_type, crate::attribute::AttrType::Nonce)?;
        if raw_attr.value.len() > NONCE_VALUE_MAX_LEN {
            return Err(crate::errors::STUNMessageError::SyntaxError(format!(
                "value length for {:?}: {} exceeds max length: {}",
                crate::attribute::AttrType::Nonce,
                raw_attr.value.len(),
                NONCE_VALUE_MAX_LEN
            )));
        }

        Ok(Self {
            value: String::from_utf8(raw_attr.value)?,
        })
    }

    fn into_raw_attr(
        self,
        _transaction_id: &[u8; crate::header::TRANSACTION_ID_LEN],
    ) -> crate::attribute::STUNRawAttribute {
        crate::attribute::STUNRawAttribute::new(self.get_type(), self.value.into_bytes())
    }
}
