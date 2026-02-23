use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use rootcause::{Report, bail};
use std::{
    fmt,
    io::{Read, Write},
};
use stun_formats::{
    MessageChecker,
    attributes::{AttributeExtDynamic, AttributeExtStatic, AttributeFactory},
    define_attribute,
};

//  0                   1                   2                   3
//  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |     Family    |         Reserved        |Class|     Number    |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                   Reason Phrase (variable)                   ..
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Clone)]
pub struct AddressErrorCodeAttrbute {
    family: u8,
    class: u8,
    number: u8,
    reason_phrase: String,
}

impl fmt::Debug for AddressErrorCodeAttrbute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("address error code")
            .field("family", &self.family)
            .field("class", &self.class)
            .field("number", &self.number)
            .field("reason_phrase", &self.reason_phrase)
            .finish()
    }
}

define_attribute!(0x8001, AddressErrorCodeAttrbute, "ADDRESS_ERROR_CODE");

impl MessageChecker for AddressErrorCodeAttrbute {}

impl AttributeFactory for AddressErrorCodeAttrbute {
    fn from_raw_attr(
        raw_attr: stun_formats::attributes::RawAttribute,
        _transaction_id: &stun_formats::header::TransactionId,
    ) -> Result<Self, Report> {
        stun_formats::attributes::check_attr_match(raw_attr.attr_type, Self::STATIC_ATTR_TYPE)?;
        let mut buffer = raw_attr.value.as_slice();
        let family = buffer.read_u8()?;
        if family != stun_formats::attributes::rfc8489::ADDRESS_FAMILY_V4
            && family != stun_formats::attributes::rfc8489::ADDRESS_FAMILY_V6
        {
            bail!(stun_formats::errors::StunMessageError::InvalidMessage(
                format!("invalid family: {}", family),
            ))
        }

        let class = buffer.read_u16::<BigEndian>()?;
        debug_assert_eq!(class & 0b000, 0);
        let class = (class & 0b111) as u8;
        let number = buffer.read_u8()?;
        let mut reason_phrase = String::new();
        let _ = buffer.read_to_string(&mut reason_phrase)?;
        Ok(Self {
            family,
            class,
            number,
            reason_phrase,
        })
    }

    fn into_raw_attr(
        self,
        _transaction_id: &stun_formats::header::TransactionId,
    ) -> stun_formats::attributes::RawAttribute {
        let mut value = Vec::with_capacity(self.reason_phrase.len().checked_add(4).unwrap());
        value.write_u8(self.family).unwrap();
        value.write_u16::<BigEndian>(self.class as u16).unwrap();
        value.write_u8(self.number).unwrap();
        value.write_all(&self.reason_phrase.into_bytes()).unwrap();
        stun_formats::attributes::RawAttribute::new(Self::STATIC_ATTR_TYPE, value)
    }
}
