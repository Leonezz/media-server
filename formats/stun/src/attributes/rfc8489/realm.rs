use std::fmt;

use utils::traits::dynamic_sized_packet::DynamicSizedPacket;

use crate::{
    MessageChecker,
    attributes::{
        AttributeExtDynamic, AttributeExtStatic, AttributeFactory, STUN_ATTRIBUTE_PADDING_SIZE,
        check_attr_match, get_after_padding_size,
    },
    define_attribute,
    errors::StunMessageResult,
};

#[derive(Clone)]
pub struct RealmAttribute {
    value: String,
}

impl RealmAttribute {
    pub fn new(value: &str) -> StunMessageResult<Self> {
        if value.len() > REALM_VALUE_MAX_LEN {
            return Err(crate::errors::StunMessageError::SyntaxError(format!(
                "value length for {:?}: {} exceeds max length: {}",
                Self::STATIC_ATTR_TYPE,
                value.len(),
                REALM_VALUE_MAX_LEN
            )));
        }

        Ok(Self {
            value: value.to_owned(),
        })
    }
}

impl DynamicSizedPacket for RealmAttribute {
    fn get_packet_bytes_count(&self) -> usize {
        get_after_padding_size(self.value.len(), STUN_ATTRIBUTE_PADDING_SIZE)
    }
}

impl fmt::Debug for RealmAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AttrType: {:?}, value: {}", self.get_type(), self.value)
    }
}

/// It MUST be a UTF-8-encoded [RFC3629] sequence of
/// fewer than 128 characters
/// (which can be as long as 509 bytes when encoding them and as long as 763 bytes when decoding them)
pub const REALM_VALUE_MAX_LEN: usize = 763;

define_attribute!(0x0014, RealmAttribute, "REALM");

impl MessageChecker for RealmAttribute {}

impl AttributeFactory for RealmAttribute {
    fn from_raw_attr(
        raw_attr: crate::attributes::RawAttribute,
        _transaction_id: &crate::header::TransactionId,
    ) -> Result<Self, crate::errors::StunMessageError> {
        check_attr_match(raw_attr.attr_type, Self::STATIC_ATTR_TYPE)?;
        if raw_attr.value.len() > REALM_VALUE_MAX_LEN {
            return Err(crate::errors::StunMessageError::SyntaxError(format!(
                "value length for {:?}: {} exceeds max length: {}",
                Self::STATIC_ATTR_TYPE,
                raw_attr.value.len(),
                REALM_VALUE_MAX_LEN
            )));
        }

        Ok(Self {
            value: String::from_utf8(raw_attr.value)?,
        })
    }

    fn into_raw_attr(
        self,
        _transaction_id: &crate::header::TransactionId,
    ) -> crate::attributes::RawAttribute {
        crate::attributes::RawAttribute::new(self.get_type(), self.value.into_bytes())
    }
}
