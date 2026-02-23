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
pub struct UserNameAttribute {
    username: String,
}

impl UserNameAttribute {
    pub fn new(username: &str) -> StunMessageResult<Self> {
        if username.len() > USERNAME_MAX_LEN {
            bail!(crate::errors::StunMessageError::SyntaxError(format!(
                "length of {:?}: {} exceeds max length {}",
                Self::STATIC_ATTR_TYPE,
                username,
                USERNAME_MAX_LEN
            )))
        }

        Ok(Self {
            username: username.to_owned(),
        })
    }

    pub fn username(&self) -> &String {
        &self.username
    }
}

impl fmt::Debug for UserNameAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AttrType: {:?}, value: {}",
            self.get_type(),
            self.username
        )
    }
}

impl DynamicSizedPacket for UserNameAttribute {
    fn get_packet_bytes_count(&self) -> usize {
        get_after_padding_size(self.username.len(), STUN_ATTRIBUTE_PADDING_SIZE)
    }
}

// The value of USERNAME is a variable-length value containing
// the authentication username.
// It MUST contain a UTF-8-encoded [RFC3629] sequence of
// fewer than 509 bytes and MUST have been processed using
// the OpaqueString profile [RFC8265].
// A compliant implementation MUST be able to parse a
// UTF-8-encoded sequence of 763 or fewer octets to be compatible with [RFC5389].
pub const USERNAME_MAX_LEN: usize = 763;

define_attribute!(0x0006, UserNameAttribute, "USER_NAME");

impl MessageChecker for UserNameAttribute {}

impl AttributeFactory for UserNameAttribute {
    fn from_raw_attr(
        raw_attr: crate::attributes::RawAttribute,
        _transaction_id: &crate::header::TransactionId,
    ) -> StunMessageResult<Self> {
        check_attr_match(raw_attr.attr_type, Self::STATIC_ATTR_TYPE).trace()?;
        let username = String::from_utf8(raw_attr.value).map_err(StunMessageError::from)?;
        if username.len() > USERNAME_MAX_LEN {
            bail!(crate::errors::StunMessageError::SyntaxError(format!(
                "length of {:?}: {} exceeds max length {}",
                Self::STATIC_ATTR_TYPE,
                username,
                USERNAME_MAX_LEN
            )))
        }
        Ok(Self { username })
    }
    fn into_raw_attr(
        self,
        _transaction_id: &crate::header::TransactionId,
    ) -> crate::attributes::RawAttribute {
        assert!(self.username.len() <= USERNAME_MAX_LEN);
        let attr_type = self.get_type();
        let value = self.username.into_bytes();
        crate::attributes::RawAttribute::new(attr_type, value)
    }
}
