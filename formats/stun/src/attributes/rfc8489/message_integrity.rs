use std::fmt;

use utils::traits::{dynamic_sized_packet::DynamicSizedPacket, writer::WriteTo};

use crate::{
    attribute::{Attribute, AttributeExt},
    attributes::{STUN_ATTRIBUTE_PADDING_SIZE, check_attr_match, get_after_padding_size},
    errors::STUNMessageResult,
    message::Message,
};

#[derive(Clone)]
pub struct MessageIntegrityAttribute {
    key: [u8; MESSAGE_INTEGRITY_LEN], // read/write
    hash_key: Vec<u8>,                // for sign
}

impl PartialEq for MessageIntegrityAttribute {
    fn eq(&self, other: &Self) -> bool {
        self.key.eq(other.key())
    }
}

impl Eq for MessageIntegrityAttribute {}

impl MessageIntegrityAttribute {
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

    pub fn new_long_term(username: &str, realm: &str, password: &str) -> Self {
        let key = format!("{}:{}:{}", username, realm, password);
        let hash = utils::cypto::hash::new_md5_hash(key.as_bytes());
        Self {
            key: Default::default(),
            hash_key: hash,
        }
    }

    pub fn sign(self, message: Message) -> Self {
        let dummy_self = Attribute::MessageIntegrity(Self::new_dummy());
        let attr_len = dummy_self.get_packet_bytes_count();
        let dummy_message = message.prepare_dummy_message_bytes(dummy_self);
        let mut bytes_to_hash = Vec::with_capacity(dummy_message.get_packet_bytes_count());
        dummy_message.write_to(&mut bytes_to_hash).unwrap();
        let hash = utils::cypto::hmac::new_sha1_hmac(&self.hash_key, &bytes_to_hash[..attr_len]);
        Self {
            key: hash.try_into().unwrap(),
            hash_key: self.hash_key,
        }
    }

    /// attributes used for check is not the attributes read from message,
    /// it should be constructed with new_short_term or new_long_term which has hash_key value
    pub fn check(self, message: &Message) -> STUNMessageResult<()> {
        if let Some(Attribute::MessageIntegrity(attr)) =
            message.get_attribute(Self::STATIC_ATTR_TYPE.unwrap())
        {
            let dummy_attributes: Vec<_> = message
                .attributes()
                .iter()
                .take_while(|item| matches!(item, Attribute::MessageIntegrity(_)))
                .cloned()
                .collect();
            let dummy_message = Message::new(*message.header(), dummy_attributes);
            let real = self.sign(dummy_message);
            if !real.eq(attr) {
                return Err(crate::errors::StunMessageError::InvalidMessage(format!(
                    "{:?} not match message: {:?}",
                    attr, message
                )));
            }
            Ok(())
        } else {
            Err(crate::errors::StunMessageError::InvalidMessage(format!(
                "{:?} attribute not found in message {:?}",
                self.get_type(),
                message
            )))
        }
    }

    pub fn key(&self) -> &[u8; MESSAGE_INTEGRITY_LEN] {
        &self.key
    }
}

pub const MESSAGE_INTEGRITY_LEN: usize = 20;

impl fmt::Debug for MessageIntegrityAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AttrType: {:?}, value: 0x{:x?}",
            self.get_type(),
            self.key
        )
    }
}

impl DynamicSizedPacket for MessageIntegrityAttribute {
    fn get_packet_bytes_count(&self) -> usize {
        get_after_padding_size(self.key.len(), STUN_ATTRIBUTE_PADDING_SIZE)
    }
}

impl AttributeExt for MessageIntegrityAttribute {
    const STATIC_ATTR_TYPE: Option<crate::attribute::AttrType> =
        Some(crate::attribute::AttrType::MessageIntegrity);
    fn get_type(&self) -> crate::attribute::AttrType {
        Self::STATIC_ATTR_TYPE.unwrap()
    }

    fn from_raw_attr(
        raw_attr: crate::attribute::RawAttribute,
        _transaction_id: &crate::header::TransactionId,
    ) -> Result<Self, crate::errors::StunMessageError> {
        check_attr_match(raw_attr.attr_type, Self::STATIC_ATTR_TYPE.unwrap())?;
        if raw_attr.value.len() != MESSAGE_INTEGRITY_LEN {
            return Err(crate::errors::StunMessageError::SyntaxError(format!(
                "length of {:?} is not {}",
                raw_attr.attr_type, MESSAGE_INTEGRITY_LEN,
            )));
        }
        Ok(Self {
            key: raw_attr.value.try_into().unwrap(),
            hash_key: vec![],
        })
    }

    fn into_raw_attr(
        self,
        _transaction_id: &crate::header::TransactionId,
    ) -> crate::attribute::RawAttribute {
        crate::attribute::RawAttribute::new(self.get_type(), self.key.into())
    }
}
