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
pub struct SoftwareAttribute {
    software: String,
}

impl SoftwareAttribute {
    pub fn new(software: &str) -> StunMessageResult<Self> {
        if software.len() > SOFTWARE_MAX_LEN {
            return Err(crate::errors::StunMessageError::SyntaxError(format!(
                "value length for {:?}: {} exceeds max length: {}",
                Self::STATIC_ATTR_TYPE,
                software.len(),
                SOFTWARE_MAX_LEN,
            )));
        }

        Ok(Self {
            software: software.to_owned(),
        })
    }

    pub fn software(&self) -> &String {
        &self.software
    }
}

impl fmt::Debug for SoftwareAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AttrType: {:?}, value: {}",
            self.get_type(),
            self.software
        )
    }
}

/// It MUST be a UTF-8-encoded [RFC3629] sequence of
/// fewer than 128 characters (which can be as long as
/// 509 when encoding them and as long as 763 bytes when decoding them)
pub const SOFTWARE_MAX_LEN: usize = 763;

impl DynamicSizedPacket for SoftwareAttribute {
    fn get_packet_bytes_count(&self) -> usize {
        get_after_padding_size(self.software.len(), STUN_ATTRIBUTE_PADDING_SIZE)
    }
}

define_attribute!(0x8022, SoftwareAttribute, "SOFTWARE");

impl MessageChecker for SoftwareAttribute {}

impl AttributeFactory for SoftwareAttribute {
    fn from_raw_attr(
        raw_attr: crate::attributes::RawAttribute,
        _transaction_id: &crate::header::TransactionId,
    ) -> Result<Self, crate::errors::StunMessageError> {
        check_attr_match(raw_attr.attr_type, Self::STATIC_ATTR_TYPE)?;
        if raw_attr.value.len() > SOFTWARE_MAX_LEN {
            return Err(crate::errors::StunMessageError::SyntaxError(format!(
                "value length for {:?}: {} exceeds max length: {}",
                Self::STATIC_ATTR_TYPE,
                raw_attr.value.len(),
                SOFTWARE_MAX_LEN,
            )));
        }
        Ok(Self {
            software: String::from_utf8(raw_attr.value)?,
        })
    }
    fn into_raw_attr(
        self,
        _transaction_id: &crate::header::TransactionId,
    ) -> crate::attributes::RawAttribute {
        crate::attributes::RawAttribute::new(self.get_type(), self.software.into_bytes())
    }
}
