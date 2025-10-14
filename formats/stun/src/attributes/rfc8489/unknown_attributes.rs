use std::{fmt, io::BufRead};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use utils::traits::dynamic_sized_packet::DynamicSizedPacket;

use crate::{
    attribute::{AttrType, AttributeExt},
    attributes::{STUN_ATTRIBUTE_PADDING_SIZE, check_attr_match, get_after_padding_size},
};

///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |        Attribute 1 Type       |        Attribute 2 Type       |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |        Attribute 3 Type       |        Attribute 4 Type     ...
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Clone)]
pub struct UnknownAttributes {
    pub attributes: Vec<AttrType>,
}

impl fmt::Debug for UnknownAttributes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AttrType: {:?}, value: {:?}",
            self.get_type(),
            self.attributes
        )
    }
}

impl DynamicSizedPacket for UnknownAttributes {
    fn get_packet_bytes_count(&self) -> usize {
        get_after_padding_size(self.attributes.len() * 2, STUN_ATTRIBUTE_PADDING_SIZE)
    }
}

impl AttributeExt for UnknownAttributes {
    fn get_type(&self) -> AttrType {
        AttrType::UnknownAttributes
    }

    fn from_raw_attr(
        raw_attr: crate::attribute::RawAttribute,
        _transaction_id: &crate::header::TransactionId,
    ) -> Result<Self, crate::errors::StunMessageError> {
        check_attr_match(raw_attr.attr_type, AttrType::UnknownAttributes)?;
        let mut attributes = Vec::with_capacity(raw_attr.value.len() / 2);
        let mut bytes = raw_attr.value.as_slice();
        while bytes.has_data_left()? {
            let attr = bytes.read_u16::<BigEndian>()?;
            if attr == 0 {
                break;
            }
            attributes.push(AttrType::from(attr));
        }
        Ok(Self { attributes })
    }

    fn into_raw_attr(
        self,
        _transaction_id: &crate::header::TransactionId,
    ) -> crate::attribute::RawAttribute {
        let mut value = Vec::with_capacity(self.attributes.len() * 2);
        self.attributes.iter().for_each(|item| {
            value.write_u16::<BigEndian>((*item).into()).unwrap();
        });
        crate::attribute::RawAttribute::new(self.get_type(), value)
    }
}
