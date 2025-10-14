// The XOR-RELAYED-ADDRESS attribute is present in Allocate responses.
// It specifies the address and port that the server allocated to the client.
// It is encoded in the same way as the XORMAPPED-ADDRESS attribute.

use std::fmt;
use stun_formats::{
    MessageChecker,
    attributes::{AttributeExtDynamic, AttributeExtStatic, AttributeFactory, check_attr_match},
    define_attribute,
};

#[derive(Clone)]
pub struct XorRelayedAddressAttribute(stun_formats::attributes::rfc8489::XorMappedAddressAttribute);

impl fmt::Debug for XorRelayedAddressAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "xor relayed address: [{}]:{}",
            self.0.address(),
            self.0.port()
        )
    }
}

define_attribute!(0x0016, XorRelayedAddressAttribute, "XOR_RELAYED_ADDRESS");

impl MessageChecker for XorRelayedAddressAttribute {}

impl AttributeFactory for XorRelayedAddressAttribute {
    fn from_raw_attr(
        raw_attr: stun_formats::attributes::RawAttribute,
        transaction_id: &stun_formats::header::TransactionId,
    ) -> Result<Self, stun_formats::errors::StunMessageError> {
        check_attr_match(raw_attr.attr_type, Self::STATIC_ATTR_TYPE)?;
        let mut buffer = raw_attr.value.as_slice();
        let address =
            stun_formats::attributes::rfc8489::XorMappedAddressAttribute::read_without_type(
                &mut buffer,
                transaction_id,
            )?;
        Ok(Self(address))
    }

    fn into_raw_attr(
        self,
        transaction_id: &stun_formats::header::TransactionId,
    ) -> stun_formats::attributes::RawAttribute {
        let mut value = Vec::new();
        self.0
            .write_without_type(&mut value, transaction_id)
            .unwrap();
        stun_formats::attributes::RawAttribute::new(Self::STATIC_ATTR_TYPE, value)
    }
}
