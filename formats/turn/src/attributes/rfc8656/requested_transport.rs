use byteorder::{BigEndian, ReadBytesExt};
use iana_formats::protocol_numbers::ProtocolNumberStatic;
use std::fmt;
use stun_formats::{
    MessageChecker,
    attributes::{AttributeExtDynamic, AttributeExtStatic, AttributeFactory},
    define_attribute,
};

//  0                   1                   2                   3
//  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |    Protocol   |                        RFFU                   |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Clone, Copy)]
pub struct RequestedTransportAttribute {
    protocol: u8,
}

const REQUESTED_TRANSPORT_ATTR_LEN: usize = 4;

impl fmt::Debug for RequestedTransportAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "requested transport: {}", self.protocol)
    }
}

define_attribute!(0x0019, RequestedTransportAttribute, "REQUESTED_TRANSPORT");

impl MessageChecker for RequestedTransportAttribute {}

impl AttributeFactory for RequestedTransportAttribute {
    fn from_raw_attr(
        raw_attr: stun_formats::attributes::RawAttribute,
        _transaction_id: &stun_formats::header::TransactionId,
    ) -> Result<Self, stun_formats::errors::StunMessageError> {
        stun_formats::attributes::check_attr_match(raw_attr.attr_type, Self::STATIC_ATTR_TYPE)?;
        if raw_attr.value.len() != REQUESTED_TRANSPORT_ATTR_LEN {
            return Err(stun_formats::errors::StunMessageError::SyntaxError(
                format!(
                    "requested transport attribute expects {} bytes, got {} bytes instead",
                    REQUESTED_TRANSPORT_ATTR_LEN,
                    raw_attr.value.len()
                ),
            ));
        }
        let mut buffer = raw_attr.value.as_slice();
        let protocol = buffer.read_u8()?;
        assert_eq!(protocol, iana_formats::protocol_numbers::UDP::DECIMAL);
        let rffu = buffer.read_u24::<BigEndian>()?;
        debug_assert_eq!(rffu, 0);
        Ok(Self { protocol })
    }

    fn into_raw_attr(
        self,
        _transaction_id: &stun_formats::header::TransactionId,
    ) -> stun_formats::attributes::RawAttribute {
        let value = vec![self.protocol, 0, 0, 0];
        stun_formats::attributes::RawAttribute::new(Self::STATIC_ATTR_TYPE, value)
    }
}
