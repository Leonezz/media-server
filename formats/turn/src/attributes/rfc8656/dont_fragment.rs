use std::fmt;
use stun_formats::{
    MessageChecker,
    attributes::{AttributeExtDynamic, AttributeExtStatic, AttributeFactory},
    define_attribute,
};

// This attribute is used by the client to request that
// the server set the DF (Don't Fragment) bit in the
// IP header when relaying the application data onward to
// the peer and for determining the server capability in Allocate requests.
// This attribute has no value part, and thus, the attribute length field is 0.
#[derive(Clone, Copy)]
pub struct DontFragmentAttribute;

impl fmt::Debug for DontFragmentAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "dont fragment")
    }
}

define_attribute!(0x001A, DontFragmentAttribute, "DONT_FRAGMENT");

impl MessageChecker for DontFragmentAttribute {}

impl AttributeFactory for DontFragmentAttribute {
    fn from_raw_attr(
        raw_attr: stun_formats::attributes::RawAttribute,
        _transaction_id: &stun_formats::header::TransactionId,
    ) -> Result<Self, stun_formats::errors::StunMessageError> {
        stun_formats::attributes::check_attr_match(raw_attr.attr_type, Self::STATIC_ATTR_TYPE)?;
        if !raw_attr.value.is_empty() {
            return Err(stun_formats::errors::StunMessageError::SyntaxError(
                format!(
                    "dont fragment attribute expects 0 bytes, got {} bytes instead",
                    raw_attr.value.len()
                ),
            ));
        }

        Ok(Self {})
    }

    fn into_raw_attr(
        self,
        _transaction_id: &stun_formats::header::TransactionId,
    ) -> stun_formats::attributes::RawAttribute {
        stun_formats::attributes::RawAttribute::new(Self::STATIC_ATTR_TYPE, vec![])
    }
}
