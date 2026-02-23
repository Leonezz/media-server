use std::fmt;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use stun_formats::{
    MessageChecker,
    attributes::{AttributeExtDynamic, AttributeExtStatic, AttributeFactory},
    define_attribute,
};

//  0                   1                   2                   3
//  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |         Channel Number        |            RFFU = 0           |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Clone, Copy)]
pub struct ChannelNumberAttribute {
    pub channel_number: u16,
}

impl fmt::Debug for ChannelNumberAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "channel number: {}", self.channel_number)
    }
}

pub const CHANNEL_NUMBER_ATTR_LEN: usize = 4;

define_attribute!(0x000C, ChannelNumberAttribute, "CHANNEL_NUMBER");

impl MessageChecker for ChannelNumberAttribute {}

impl AttributeFactory for ChannelNumberAttribute {
    fn from_raw_attr(
        raw_attr: stun_formats::attributes::RawAttribute,
        _transaction_id: &stun_formats::header::TransactionId,
    ) -> Result<Self, stun_formats::errors::StunMessageError> {
        stun_formats::attributes::check_attr_match(raw_attr.attr_type, Self::STATIC_ATTR_TYPE)?;
        if raw_attr.value.len() != CHANNEL_NUMBER_ATTR_LEN {
            return Err(stun_formats::errors::StunMessageError::SyntaxError(
                format!(
                    "channel number attribute expects {} bytes of value, got {} bytes instead",
                    CHANNEL_NUMBER_ATTR_LEN,
                    raw_attr.value.len()
                ),
            ));
        }
        let mut buffer = raw_attr.value.as_slice();
        let channel_number = buffer.read_u16::<BigEndian>()?;
        let rffu = buffer.read_u16::<BigEndian>()?;
        debug_assert_eq!(rffu, 0);
        Ok(Self { channel_number })
    }

    fn into_raw_attr(
        self,
        _transaction_id: &stun_formats::header::TransactionId,
    ) -> stun_formats::attributes::RawAttribute {
        let mut value = Vec::with_capacity(CHANNEL_NUMBER_ATTR_LEN);
        value.write_u16::<BigEndian>(self.channel_number).unwrap();
        value.write_u16::<BigEndian>(0).unwrap();
        stun_formats::attributes::RawAttribute::new(Self::STATIC_ATTR_TYPE, value)
    }
}
