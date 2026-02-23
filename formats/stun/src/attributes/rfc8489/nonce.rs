use std::fmt;

use rootcause::bail;
use utils::{errors::context::LocationExt, traits::dynamic_sized_packet::DynamicSizedPacket};

use crate::{
    MessageChecker,
    attributes::{
        AttributeExtDynamic, AttributeExtStatic, AttributeFactory, STUN_ATTRIBUTE_PADDING_SIZE,
        check_attr_match, get_after_padding_size,
    },
    define_attribute,
    errors::{StunMessageError, StunMessageResult},
};

#[derive(Clone)]
pub struct NonceAttribute {
    value: String,
}

impl NonceAttribute {
    pub fn new(value: &str) -> StunMessageResult<Self> {
        if value.len() > NONCE_VALUE_MAX_LEN {
            bail!(crate::errors::StunMessageError::SyntaxError(format!(
                "value length for {:?}: {} exceeds max length: {}",
                Self::STATIC_ATTR_TYPE,
                value.len(),
                NONCE_VALUE_MAX_LEN
            )))
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

define_attribute!(0x0015, NonceAttribute, "NONCE");

impl MessageChecker for NonceAttribute {}

impl AttributeFactory for NonceAttribute {
    fn from_raw_attr(
        raw_attr: crate::attributes::RawAttribute,
        _transaction_id: &crate::header::TransactionId,
    ) -> StunMessageResult<Self> {
        check_attr_match(raw_attr.attr_type, Self::STATIC_ATTR_TYPE).trace()?;
        if raw_attr.value.len() > NONCE_VALUE_MAX_LEN {
            bail!(crate::errors::StunMessageError::SyntaxError(format!(
                "value length for {:?}: {} exceeds max length: {}",
                Self::STATIC_ATTR_TYPE,
                raw_attr.value.len(),
                NONCE_VALUE_MAX_LEN
            )))
        }

        Ok(Self {
            value: String::from_utf8(raw_attr.value).map_err(StunMessageError::from)?,
        })
    }
    fn into_raw_attr(
        self,
        _transaction_id: &crate::header::TransactionId,
    ) -> crate::attributes::RawAttribute {
        crate::attributes::RawAttribute::new(self.get_type(), self.value.into_bytes())
    }
}
