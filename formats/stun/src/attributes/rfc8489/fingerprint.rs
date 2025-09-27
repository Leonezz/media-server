use std::fmt;

use utils::traits::{
    dynamic_sized_packet::DynamicSizedPacket, fixed_packet::FixedPacket, writer::WriteTo,
};

use crate::{
    attribute::{STUNAttribute, STUNAttributeExt},
    attributes::check_attr_match,
    errors::STUNMessageResult,
    message::STUNMessage,
};

#[derive(Clone, PartialEq, Eq, Copy)]
pub struct FingerPrintAttribute {
    fingerprint: u32,
}

impl FingerPrintAttribute {
    pub fn new_dummy() -> Self {
        Self { fingerprint: 0 }
    }

    pub fn sign(message: STUNMessage) -> Self {
        let dummy_self = STUNAttribute::FingerPrint(Self::new_dummy());
        let attr_length = dummy_self.get_packet_bytes_count();
        let dummy_message = message.prepare_dummy_message_bytes(dummy_self);
        let mut bytes_to_hash = Vec::with_capacity(dummy_message.get_packet_bytes_count());
        dummy_message.write_to(&mut bytes_to_hash).unwrap();
        let checksum = utils::cypto::crc::new_crc32(&bytes_to_hash[..attr_length]);
        Self {
            fingerprint: checksum ^ FINGERPRINT_XOR_VALUE,
        }
    }

    pub fn finger_print(&self) -> u32 {
        self.fingerprint
    }

    pub fn check(&self, message: &STUNMessage) -> STUNMessageResult<()> {
        if let Some(STUNAttribute::FingerPrint(fingerprint)) = message.attributes().last()
            && fingerprint == self
        {
            let mut dummy_attributes = message.attributes().clone();
            dummy_attributes.pop();
            let dummy_message = STUNMessage::new(*message.header(), dummy_attributes);
            let real = Self::sign(dummy_message);
            if &real != self {
                return Err(crate::errors::STUNMessageError::InvalidMessage(format!(
                    "finger print not match, expected: 0x{:x}, real: 0x{:x}",
                    self.fingerprint, real.fingerprint
                )));
            }
            Ok(())
        } else {
            Err(crate::errors::STUNMessageError::InvalidMessage(format!(
                "last attribute of message not match: {:?} -> {:?}",
                message, self
            )))
        }
    }
}

impl fmt::Debug for FingerPrintAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AttrType: {:?}, value: 0x{:x}",
            self.get_type(),
            self.fingerprint
        )
    }
}

// The value of the attribute is computed as the CRC-32
// of the STUN message up to (but excluding)
// the FINGERPRINT attribute itself,
// XOR’ed with the 32-bit value 0x5354554e
pub const FINGERPRINT_XOR_VALUE: u32 = 0x5354554e;
pub const FINGERPRINT_LEN: usize = 4;

impl FixedPacket for FingerPrintAttribute {
    fn bytes_count() -> usize {
        FINGERPRINT_LEN
    }
}

impl STUNAttributeExt for FingerPrintAttribute {
    fn get_type(&self) -> crate::attribute::AttrType {
        crate::attribute::AttrType::FingerPrint
    }

    fn from_raw_attr(
        raw_attr: crate::attribute::STUNRawAttribute,
        _transaction_id: &[u8; crate::header::TRANSACTION_ID_LEN],
    ) -> Result<Self, crate::errors::STUNMessageError> {
        check_attr_match(raw_attr.attr_type, crate::attribute::AttrType::FingerPrint)?;
        if raw_attr.value.len() != Self::bytes_count() {
            return Err(crate::errors::STUNMessageError::SyntaxError(format!(
                "value length for {:?} is not 4: {}",
                crate::attribute::AttrType::FingerPrint,
                raw_attr.value.len(),
            )));
        }

        let fingerprint = u32::from_be_bytes(raw_attr.value.try_into().unwrap());
        Ok(Self { fingerprint })
    }

    fn into_raw_attr(
        self,
        _transaction_id: &[u8; crate::header::TRANSACTION_ID_LEN],
    ) -> crate::attribute::STUNRawAttribute {
        crate::attribute::STUNRawAttribute::new(
            self.get_type(),
            self.fingerprint.to_be_bytes().to_vec(),
        )
    }
}
