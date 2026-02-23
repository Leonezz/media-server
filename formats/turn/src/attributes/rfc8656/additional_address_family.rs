use std::fmt;
use stun_formats::{
    MessageChecker,
    attributes::{AttributeExtDynamic, AttributeExtStatic, AttributeFactory},
    define_attribute,
};

use crate::attributes::rfc8656;

// This attribute is used by clients to request the allocation of
// an IPv4 and IPv6 address type from a server.
// It is encoded in the same way as the REQUESTED-ADDRESS-FAMILY attribute;
// see Section 18.6. The ADDITIONAL-ADDRESS-FAMILY attribute be present
// in the Allocate request. The attribute value of 0x02 (IPv6 address) is the only valid value in Allocate request.
#[derive(Clone, Copy)]
pub struct AdditionalAddressFamilyAttribute(
    crate::attributes::rfc8656::RequestedAddressFamilyAttribute,
);

define_attribute!(
    0x8000,
    AdditionalAddressFamilyAttribute,
    "ADDITIONAL_ADDRESS_FAMILY"
);

const ADDITIONAL_ADDRESS_FAMILY_ATTR_LEN: usize = 4;

impl fmt::Debug for AdditionalAddressFamilyAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "additional address family: {}", self.0.family())
    }
}

impl MessageChecker for AdditionalAddressFamilyAttribute {
    fn check_request(
        &self,
        message: &stun_formats::message::Message,
    ) -> stun_formats::errors::StunMessageResult<()> {
        if message
            .get_attribute(rfc8656::RequestedAddressFamilyAttribute::STATIC_ATTR_TYPE)
            .is_some()
        {
            return Err(stun_formats::errors::StunMessageError::InvalidMessage(
                format!(
                    "request with {} attribute cannot have {} attribute also",
                    Self::STATIC_NAME,
                    rfc8656::RequestedAddressFamilyAttribute::STATIC_NAME
                ),
            ));
        }
        Ok(())
    }
}

impl AttributeFactory for AdditionalAddressFamilyAttribute {
    fn from_raw_attr(
        raw_attr: stun_formats::attributes::RawAttribute,
        _transaction_id: &stun_formats::header::TransactionId,
    ) -> Result<Self, stun_formats::errors::StunMessageError> {
        stun_formats::attributes::check_attr_match(raw_attr.attr_type, Self::STATIC_ATTR_TYPE)?;
        if raw_attr.value.len() != ADDITIONAL_ADDRESS_FAMILY_ATTR_LEN {
            return Err(stun_formats::errors::StunMessageError::SyntaxError(
                format!(
                    "additional address family attribute expects {} bytes, got {} bytes instead",
                    ADDITIONAL_ADDRESS_FAMILY_ATTR_LEN,
                    raw_attr.value.len()
                ),
            ));
        }
        let family =
            crate::attributes::rfc8656::RequestedAddressFamilyAttribute::read_without_type(
                &mut raw_attr.value.as_slice(),
            )?;
        Ok(Self(family))
    }

    fn into_raw_attr(
        self,
        _transaction_id: &stun_formats::header::TransactionId,
    ) -> stun_formats::attributes::RawAttribute {
        let mut value = Vec::with_capacity(ADDITIONAL_ADDRESS_FAMILY_ATTR_LEN);
        self.0.write_without_type(&mut value).unwrap();
        stun_formats::attributes::RawAttribute::new(Self::STATIC_ATTR_TYPE, value)
    }
}
