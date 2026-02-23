use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use rootcause::{Report, bail};
use std::fmt;
use stun_formats::{
    MessageChecker,
    attributes::{AttributeExtDynamic, AttributeExtStatic, AttributeFactory},
    define_attribute,
};

//  0                   1                   2                   3
//  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |           Reserved            |  ICMP Type  |    ICMP Code    |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                           Error Data                          |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Clone, Copy)]
pub struct IcmpAttribute {
    icmp_type: u8,
    icmp_code: u8,
    error_data: u32,
}

impl IcmpAttribute {
    pub fn new(icmp_type: u8, icmp_code: u8, error_data: u32) -> Self {
        Self {
            icmp_type,
            icmp_code,
            error_data,
        }
    }
}

const ICMP_ATTR_LEN: usize = 8;

impl fmt::Debug for IcmpAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ICMP")
            .field("type", &self.icmp_type)
            .field("code", &self.icmp_code)
            .field("data", &self.error_data)
            .finish()
    }
}

define_attribute!(0x8004, IcmpAttribute, "ICMP");

impl MessageChecker for IcmpAttribute {}

impl AttributeFactory for IcmpAttribute {
    fn from_raw_attr(
        raw_attr: stun_formats::attributes::RawAttribute,
        _transaction_id: &stun_formats::header::TransactionId,
    ) -> Result<Self, Report> {
        stun_formats::attributes::check_attr_match(raw_attr.attr_type, Self::STATIC_ATTR_TYPE)?;
        if raw_attr.value.len() != ICMP_ATTR_LEN {
            bail!(stun_formats::errors::StunMessageError::SyntaxError(
                format!(
                    "icmp attribute expects {} bytes, got {} bytes instead",
                    ICMP_ATTR_LEN,
                    raw_attr.value.len()
                ),
            ))
        }

        let mut buffer = raw_attr.value.as_slice();
        let reserved = buffer.read_u16::<BigEndian>()?;
        debug_assert_eq!(reserved, 0);
        let icmp_type = buffer.read_u8()?;
        let icmp_code = buffer.read_u8()?;
        let error_data = buffer.read_u32::<BigEndian>()?;
        Ok(Self {
            icmp_type,
            icmp_code,
            error_data,
        })
    }

    fn into_raw_attr(
        self,
        _transaction_id: &stun_formats::header::TransactionId,
    ) -> stun_formats::attributes::RawAttribute {
        let mut value = Vec::with_capacity(ICMP_ATTR_LEN);
        value.write_u16::<BigEndian>(0).unwrap();
        value.write_u8(self.icmp_type).unwrap();
        value.write_u8(self.icmp_code).unwrap();
        value.write_u32::<BigEndian>(self.error_data).unwrap();
        stun_formats::attributes::RawAttribute::new(Self::STATIC_ATTR_TYPE, value)
    }
}
