use std::fmt;

use utils::traits::fixed_packet::FixedPacket;

use crate::{attribute::AttributeExt, attributes::check_attr_match};

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

impl AttributeExt for UserHashAttribute {
    fn get_type(&self) -> crate::attribute::AttrType {
        crate::attribute::AttrType::UserHash
    }
    fn from_raw_attr(
        raw_attr: crate::attribute::RawAttribute,
        _transaction_id: &crate::header::TransactionId,
    ) -> Result<Self, crate::errors::StunMessageError> {
        check_attr_match(raw_attr.attr_type, crate::attribute::AttrType::UserHash)?;
        if raw_attr.value.len() != USERHASH_LEN {
            return Err(crate::errors::StunMessageError::SyntaxError(format!(
                "user hash value should be of 32 bytes, got {} bytes",
                raw_attr.value.len()
            )));
        }
        Ok(Self {
            userhash: raw_attr.value.try_into().unwrap(),
        })
    }

    fn into_raw_attr(
        self,
        _transaction_id: &crate::header::TransactionId,
    ) -> crate::attribute::RawAttribute {
        crate::attribute::RawAttribute::new(self.get_type(), self.userhash.into())
    }
}
