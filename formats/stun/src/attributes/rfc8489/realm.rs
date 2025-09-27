use std::fmt;

use utils::traits::dynamic_sized_packet::DynamicSizedPacket;

use crate::{
    attribute::STUNAttributeExt,
    attributes::{STUN_ATTRIBUTE_PADDING_SIZE, check_attr_match, get_after_padding_size},
    errors::STUNMessageResult,
};

#[derive(Clone)]
pub struct RealmAttribute {
    value: String,
}

impl RealmAttribute {
    pub fn new(value: &str) -> STUNMessageResult<Self> {
        if value.len() > REALM_VALUE_MAX_LEN {
            return Err(crate::errors::STUNMessageError::SyntaxError(format!(
                "value length for {:?}: {} exceeds max length: {}",
                crate::attribute::AttrType::Realm,
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

impl STUNAttributeExt for RealmAttribute {
    fn get_type(&self) -> crate::attribute::AttrType {
        crate::attribute::AttrType::Realm
    }

    fn from_raw_attr(
        raw_attr: crate::attribute::STUNRawAttribute,
        _transaction_id: &[u8; crate::header::TRANSACTION_ID_LEN],
    ) -> Result<Self, crate::errors::STUNMessageError> {
        check_attr_match(raw_attr.attr_type, crate::attribute::AttrType::Realm)?;
        if raw_attr.value.len() > REALM_VALUE_MAX_LEN {
            return Err(crate::errors::STUNMessageError::SyntaxError(format!(
                "value length for {:?}: {} exceeds max length: {}",
                crate::attribute::AttrType::Realm,
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
        _transaction_id: &[u8; crate::header::TRANSACTION_ID_LEN],
    ) -> crate::attribute::STUNRawAttribute {
        crate::attribute::STUNRawAttribute::new(self.get_type(), self.value.into_bytes())
    }
}
