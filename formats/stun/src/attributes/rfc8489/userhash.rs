use std::fmt;

use rootcause::bail;
use utils::{errors::context::LocationExt, traits::fixed_packet::FixedPacket};

use crate::{
    MessageChecker,
    attributes::{AttributeExtDynamic, AttributeExtStatic, AttributeFactory, check_attr_match},
    define_attribute,
    errors::StunMessageResult,
};

// The value of USERHASH has a fixed length of 32 bytes.
pub const USERHASH_LEN: usize = 32;

#[derive(Clone)]
pub struct UserHashAttribute {
    userhash: [u8; USERHASH_LEN],
}

impl UserHashAttribute {
    pub fn new(username: &str, realm: &str) -> Self {
        let key = format!("{}:{}", username, realm);
        let hash = utils::cypto::hash::new_sha256_hash(key.as_bytes());
        Self {
            userhash: hash.try_into().unwrap(),
        }
    }
}

impl fmt::Debug for UserHashAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AttrType: {:?}, value: 0x{:x?}",
            self.get_type(),
            self.userhash
        )
    }
}

impl FixedPacket for UserHashAttribute {
    fn bytes_count() -> usize {
        USERHASH_LEN
    }
}

define_attribute!(0x001E, UserHashAttribute, "USER_HASH");

impl MessageChecker for UserHashAttribute {}

impl AttributeFactory for UserHashAttribute {
    fn from_raw_attr(
        raw_attr: crate::attributes::RawAttribute,
        _transaction_id: &crate::header::TransactionId,
    ) -> StunMessageResult<Self> {
        check_attr_match(raw_attr.attr_type, Self::STATIC_ATTR_TYPE).trace()?;
        if raw_attr.value.len() != USERHASH_LEN {
            bail!(crate::errors::StunMessageError::SyntaxError(format!(
                "user hash value should be of 32 bytes, got {} bytes",
                raw_attr.value.len()
            )))
        }
        Ok(Self {
            userhash: raw_attr.value.try_into().unwrap(),
        })
    }

    fn into_raw_attr(
        self,
        _transaction_id: &crate::header::TransactionId,
    ) -> crate::attributes::RawAttribute {
        crate::attributes::RawAttribute::new(self.get_type(), self.userhash.into())
    }
}
