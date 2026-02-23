use byteorder::{BigEndian, ReadBytesExt};
use iana_formats::addrress_family::AddressFamilyStatic;
use rootcause::{Report, bail};
use std::{fmt, io};
use stun_formats::{
    MessageChecker,
    attributes::{AttributeExtDynamic, AttributeExtStatic, AttributeFactory},
    define_attribute,
    errors::StunMessageResult,
};

use crate::attributes::rfc8656;

//  0                   1                   2                   3
//  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |     Family    |                     Reserved                  |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Clone, Copy)]
pub struct RequestedAddressFamilyAttribute {
    family: iana_formats::addrress_family::AddressFamily,
}

impl RequestedAddressFamilyAttribute {
    pub fn family(&self) -> iana_formats::addrress_family::AddressFamily {
        self.family
    }
    pub fn read_without_type<R: io::Read>(reader: &mut R) -> StunMessageResult<Self> {
        let family = reader.read_u8()?;
        if family as u16 != iana_formats::addrress_family::IPv4::DECIMAL
            && family as u16 != iana_formats::addrress_family::IPv6::DECIMAL
        {
            bail!(stun_formats::errors::StunMessageError::InvalidMessage(
                format!("invalid family: {}", family),
            ))
        }
        let family = iana_formats::addrress_family::from_number(family as u16)
            .unwrap()
            .address_family();
        let reserved = reader.read_u24::<BigEndian>()?;
        debug_assert_eq!(reserved, 0);
        Ok(Self { family })
    }
    pub fn write_without_type<W: io::Write>(&self, writer: &mut W) -> StunMessageResult<()> {
        writer.write_all(&[self.family.inner() as u8, 0, 0, 0])?;
        Ok(())
    }
}

const REQUESTED_ADDRESS_FAMILY_ATTR_LEN: usize = 4;

impl fmt::Debug for RequestedAddressFamilyAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "requested address family: {}", self.family)
    }
}

define_attribute!(
    0x0017,
    RequestedAddressFamilyAttribute,
    "REQUESTED_ADDRESS_FAMILY"
);

impl MessageChecker for RequestedAddressFamilyAttribute {
    fn check_request(&self, message: &stun_formats::message::Message) -> StunMessageResult<()> {
        if message
            .get_attribute(rfc8656::AdditionalAddressFamilyAttribute::STATIC_ATTR_TYPE)
            .is_some()
        {
            bail!(stun_formats::errors::StunMessageError::InvalidMessage(
                format!(
                    "request with {} attribute cannot have {} attribute also",
                    Self::STATIC_NAME,
                    rfc8656::AdditionalAddressFamilyAttribute::STATIC_NAME
                ),
            ))
        }
        Ok(())
    }
}

impl AttributeFactory for RequestedAddressFamilyAttribute {
    fn from_raw_attr(
        raw_attr: stun_formats::attributes::RawAttribute,
        _transaction_id: &stun_formats::header::TransactionId,
    ) -> Result<Self, Report> {
        stun_formats::attributes::check_attr_match(raw_attr.attr_type, Self::STATIC_ATTR_TYPE)?;
        if raw_attr.value.len() != REQUESTED_ADDRESS_FAMILY_ATTR_LEN {
            bail!(stun_formats::errors::StunMessageError::SyntaxError(
                format!(
                    "requested address family attribute expects {} bytes, got {} bytes instead",
                    REQUESTED_ADDRESS_FAMILY_ATTR_LEN,
                    raw_attr.value.len()
                ),
            ))
        }

        let mut buffer = raw_attr.value.as_slice();
        Self::read_without_type(&mut buffer)
    }

    fn into_raw_attr(
        self,
        _transaction_id: &stun_formats::header::TransactionId,
    ) -> stun_formats::attributes::RawAttribute {
        let value = vec![self.family.inner() as u8, 0, 0, 0];
        stun_formats::attributes::RawAttribute::new(Self::STATIC_ATTR_TYPE, value)
    }
}
