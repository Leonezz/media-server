use std::fmt;

use utils::traits::{dynamic_sized_packet::DynamicSizedPacket, writer::WriteTo};

use crate::{
    attribute::{STUNAttribute, STUNAttributeExt},
    attributes::{STUN_ATTRIBUTE_PADDING_SIZE, check_attr_match, get_after_padding_size},
    errors::STUNMessageResult,
    message::STUNMessage,
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

    pub fn sign(self, message: STUNMessage) -> Self {
        let dummy_self = STUNAttribute::MessageIntegrity(Self::new_dummy());
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
    pub fn check(self, message: &STUNMessage) -> STUNMessageResult<()> {
        if let Some(STUNAttribute::MessageIntegrity(attr)) =
            message.get_attribute(crate::attribute::AttrType::MessageIntegrity)
        {
            let dummy_attributes: Vec<_> = message
                .attributes()
                .iter()
                .take_while(|item| matches!(item, STUNAttribute::MessageIntegrity(_)))
                .cloned()
                .collect();
            let dummy_message = STUNMessage::new(*message.header(), dummy_attributes);
            let real = self.sign(dummy_message);
            if !real.eq(attr) {
                return Err(crate::errors::STUNMessageError::InvalidMessage(format!(
                    "{:?} not match message: {:?}",
                    attr, message
                )));
            }
            Ok(())
        } else {
            Err(crate::errors::STUNMessageError::InvalidMessage(format!(
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

impl STUNAttributeExt for MessageIntegrityAttribute {
    fn get_type(&self) -> crate::attribute::AttrType {
        crate::attribute::AttrType::MessageIntegrity
    }

    fn from_raw_attr(
        raw_attr: crate::attribute::STUNRawAttribute,
        _transaction_id: &[u8; crate::header::TRANSACTION_ID_LEN],
    ) -> Result<Self, crate::errors::STUNMessageError> {
        check_attr_match(
            raw_attr.attr_type,
            crate::attribute::AttrType::MessageIntegrity,
        )?;
        if raw_attr.value.len() != MESSAGE_INTEGRITY_LEN {
            return Err(crate::errors::STUNMessageError::SyntaxError(format!(
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
        _transaction_id: &[u8; crate::header::TRANSACTION_ID_LEN],
    ) -> crate::attribute::STUNRawAttribute {
        crate::attribute::STUNRawAttribute::new(self.get_type(), self.key.into())
    }
}
