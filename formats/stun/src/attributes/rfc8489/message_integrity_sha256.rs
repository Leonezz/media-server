use std::fmt;

use utils::traits::{dynamic_sized_packet::DynamicSizedPacket, writer::WriteTo};

use crate::{
    attribute::{STUNAttribute, STUNAttributeExt},
    attributes::{STUN_ATTRIBUTE_PADDING_SIZE, check_attr_match, get_after_padding_size},
    errors::STUNMessageResult,
    message::STUNMessage,
};

#[derive(Clone)]
pub struct MessageIntegritySHA256Attribute {
    key: Vec<u8>,
    hash_key: Vec<u8>,
}

impl PartialEq for MessageIntegritySHA256Attribute {
    fn eq(&self, other: &Self) -> bool {
        self.key.eq(other.key())
    }
}

impl Eq for MessageIntegritySHA256Attribute {}

impl MessageIntegritySHA256Attribute {
    pub fn new_dummy() -> Self {
        Self {
            key: Default::default(),
            hash_key: Default::default(),
        }
    }

    pub fn new_short_term(password: &str) -> Self {
        Self {
            key: Default::default(),
            hash_key: password.as_bytes().into(),
        }
    }

    pub fn new_legacy_long_term(username: &str, realm: &str, password: &str) -> Self {
        let key = format!("{}:{}:{}", username, realm, password);
        let hash_key = utils::cypto::hash::new_md5_hash(key.as_bytes());
        Self {
            key: Default::default(),
            hash_key,
        }
    }

    pub fn new_long_term(
        username: &str,
        realm: &str,
        password: &str,
        iter: usize,
        out_len: usize,
    ) -> Self {
        let key = format!("{}:{}:{}", username, realm, password);
        let hash_key = utils::cypto::hmac::new_pbkdf2_sha256_hmac(
            key.as_bytes(),
            realm.as_bytes(),
            iter,
            out_len,
        );
        Self {
            key: Default::default(),
            hash_key,
        }
    }

    pub fn sign(self, message: STUNMessage) -> Self {
        let dummy_self = STUNAttribute::MessageIntegritySHA256(Self::new_dummy());
        let attr_len = dummy_self.get_packet_bytes_count();
        let dummy_message = message.prepare_dummy_message_bytes(dummy_self);
        let mut bytes_to_hash = Vec::with_capacity(dummy_message.get_packet_bytes_count());
        dummy_message.write_to(&mut bytes_to_hash).unwrap();
        let hash = utils::cypto::hmac::new_sha256_hmac(&self.hash_key, &bytes_to_hash[..attr_len]);
        Self {
            key: hash,
            hash_key: self.hash_key,
        }
    }

    /// attributes used for check is not the attributes read from message,
    /// it should be constructed with new_short_term or new_long_term which has hash_key value
    pub fn check(self, message: &STUNMessage) -> STUNMessageResult<()> {
        if let Some(STUNAttribute::MessageIntegritySHA256(attr)) =
            message.get_attribute(crate::attribute::AttrType::MessageIntegritySHA256)
        {
            let dummy_attributes = message
                .attributes()
                .iter()
                .take_while(|item| matches!(item, STUNAttribute::MessageIntegritySHA256(_)))
                .cloned()
                .collect();
            let dummy_message = STUNMessage::new(*message.header(), dummy_attributes);
            let real = self.sign(dummy_message);
            if real.ne(attr) {
                return Err(crate::errors::STUNMessageError::InvalidMessage(format!(
                    "{:?} not match message: {:?}",
                    attr, message
                )));
            }
            Ok(())
        } else {
            Err(crate::errors::STUNMessageError::InvalidMessage(format!(
                "{:?} not found in message: {:?}",
                self.get_type(),
                message
            )))
        }
    }

    pub fn key(&self) -> &Vec<u8> {
        &self.key
    }
}

impl fmt::Debug for MessageIntegritySHA256Attribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AttrType: {:?}, value: 0x{:x?}",
            self.get_type(),
            self.key
        )
    }
}

impl DynamicSizedPacket for MessageIntegritySHA256Attribute {
    fn get_packet_bytes_count(&self) -> usize {
        get_after_padding_size(self.key.len(), STUN_ATTRIBUTE_PADDING_SIZE)
    }
}

// The value will be at most 32 bytes,
// but it MUST be at least 16 bytes and MUST be a multiple of 4 bytes.
// The value must be the full 32 bytes unless the STUN Usage explicitly
// specifies that truncation is allowed.
// STUN Usages may specify a minimum length longer than 16 bytes.
pub const MESSAGE_INTEGRITY_SHA256_MIN_LEN: usize = 16;
pub const MESSAGE_INTEGRITY_SHA256_MAX_LEN: usize = 32;

impl STUNAttributeExt for MessageIntegritySHA256Attribute {
    fn get_type(&self) -> crate::attribute::AttrType {
        crate::attribute::AttrType::MessageIntegritySHA256
    }

    fn from_raw_attr(
        raw_attr: crate::attribute::STUNRawAttribute,
        _transaction_id: &crate::header::TransactionId,
    ) -> Result<Self, crate::errors::STUNMessageError> {
        check_attr_match(
            raw_attr.attr_type,
            crate::attribute::AttrType::MessageIntegritySHA256,
        )?;
        if raw_attr.value.len() < MESSAGE_INTEGRITY_SHA256_MIN_LEN
            || raw_attr.value.len() > MESSAGE_INTEGRITY_SHA256_MAX_LEN
            || !raw_attr.value.len().is_multiple_of(4)
        {
            return Err(crate::errors::STUNMessageError::SyntaxError(format!(
                "length of value for {:?} is not valid: {}",
                crate::attribute::AttrType::MessageIntegritySHA256,
                raw_attr.value.len()
            )));
        }

        Ok(Self {
            key: raw_attr.value,
            hash_key: Default::default(),
        })
    }

    fn into_raw_attr(
        self,
        _transaction_id: &crate::header::TransactionId,
    ) -> crate::attribute::STUNRawAttribute {
        assert!(self.key.len() >= MESSAGE_INTEGRITY_SHA256_MIN_LEN);
        assert!(self.key.len() <= MESSAGE_INTEGRITY_SHA256_MAX_LEN);
        crate::attribute::STUNRawAttribute::new(self.get_type(), self.key)
    }
}
