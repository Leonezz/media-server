use rootcause::{Report, bail};
use std::fmt;
use stun_formats::{
    MessageChecker,
    attributes::{AttributeExtDynamic, AttributeExtStatic, AttributeFactory},
    define_attribute,
};

use crate::attributes::rfc8656;

//  0
//  0 1 2 3 4 5 6 7
// +-+-+-+-+-+-+-+-+
// |R|     RFFU    |
// +-+-+-+-+-+-+-+-+
#[derive(Clone, Copy)]
pub struct EvenPortAttribute {
    pub r: bool,
}

const EVEN_PORT_ATTR_LEN: usize = 1;

impl fmt::Debug for EvenPortAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "even port: {}", self.r)
    }
}

define_attribute!(0x0018, EvenPortAttribute, "EVEN_PORT");

impl MessageChecker for EvenPortAttribute {
    fn check_request(
        &self,
        message: &stun_formats::message::Message,
    ) -> stun_formats::errors::StunMessageResult<()> {
        if self.r
            && message
                .get_attribute(rfc8656::AdditionalAddressFamilyAttribute::STATIC_ATTR_TYPE)
                .is_some()
        {
            bail!(stun_formats::errors::StunMessageError::InvalidMessage(
                format!(
                    "{} request with {} attribute of r={} cannot have {} also",
                    Self::STATIC_NAME,
                    rfc8656::EvenPortAttribute::STATIC_NAME,
                    true,
                    rfc8656::AdditionalAddressFamilyAttribute::STATIC_NAME
                ),
            ))
        }
        Ok(())
    }
}

impl AttributeFactory for EvenPortAttribute {
    fn from_raw_attr(
        raw_attr: stun_formats::attributes::RawAttribute,
        _transaction_id: &stun_formats::header::TransactionId,
    ) -> Result<Self, Report> {
        stun_formats::attributes::check_attr_match(raw_attr.attr_type, Self::STATIC_ATTR_TYPE)?;
        if raw_attr.value.len() != EVEN_PORT_ATTR_LEN {
            bail!(stun_formats::errors::StunMessageError::SyntaxError(
                format!(
                    "event port attribute expects {} bytes, got {} bytes instead",
                    EVEN_PORT_ATTR_LEN,
                    raw_attr.value.len()
                ),
            ))
        }
        let byte = raw_attr.value[0];
        debug_assert_eq!(byte, 0b1000_0000);
        let r = ((byte >> 7) & 0b1) == 0b1;
        Ok(Self { r })
    }

    fn into_raw_attr(
        self,
        _transaction_id: &stun_formats::header::TransactionId,
    ) -> stun_formats::attributes::RawAttribute {
        let value = vec![(self.r as u8) << 7];
        stun_formats::attributes::RawAttribute::new(Self::STATIC_ATTR_TYPE, value)
    }
}
