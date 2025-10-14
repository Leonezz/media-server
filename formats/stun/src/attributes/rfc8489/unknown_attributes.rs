use std::{fmt, io::BufRead};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use utils::traits::dynamic_sized_packet::DynamicSizedPacket;

use crate::{
    MessageChecker,
    attributes::{
        AttributeExtDynamic, AttributeExtStatic, AttributeFactory, STUN_ATTRIBUTE_PADDING_SIZE,
        get_after_padding_size, rfc8489::ErrorCodeAttribute,
    },
    define_attribute,
    error_codes::{self, ErrorCodeExtStatic},
    header::MessageClass,
};

///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |        Attribute 1 Type       |        Attribute 2 Type       |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |        Attribute 3 Type       |        Attribute 4 Type     ...
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Clone)]
pub struct UnknownAttributesAttribute {
    pub attributes: Vec<u16>,
}

impl fmt::Debug for UnknownAttributesAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AttrType: {:?}, value: {:?}",
            self.get_type(),
            self.attributes
        )
    }
}

impl DynamicSizedPacket for UnknownAttributesAttribute {
    fn get_packet_bytes_count(&self) -> usize {
        get_after_padding_size(self.attributes.len() * 2, STUN_ATTRIBUTE_PADDING_SIZE)
    }
}

define_attribute!(0x000A, UnknownAttributesAttribute, "UNKNOWN_ATTRIBUTES");

impl MessageChecker for UnknownAttributesAttribute {
    fn check(&self, message: &crate::message::Message) -> crate::errors::StunMessageResult<()> {
        let class = message.message_class();
        if !matches!(class, MessageClass::ErrorResponse) {
            return Err(crate::errors::StunMessageError::InvalidMessage(format!(
                "{:?} in {:?} message is not allowed",
                Self::STATIC_ATTR_TYPE,
                class
            )));
        }
        if let Some(error_code) = message.get_attribute_ext::<ErrorCodeAttribute>() {
            if error_code.error_code().code()
                != error_codes::rfc8489::UNKNOWN_ATTRIBUTE::STATIC_CODE
            {
                return Err(crate::errors::StunMessageError::InvalidMessage(format!(
                    "code {:?} is required for message with {:?}, got {:?} instead",
                    error_codes::rfc8489::UNKNOWN_ATTRIBUTE::default(),
                    Self::STATIC_ATTR_TYPE,
                    error_code.error_code()
                )));
            }
        } else {
            return Err(crate::errors::StunMessageError::InvalidMessage(format!(
                "{:?} is required for message with {:?}",
                ErrorCodeAttribute::STATIC_ATTR_TYPE,
                Self::STATIC_ATTR_TYPE
            )));
        }
        Ok(())
    }
}

impl AttributeFactory for UnknownAttributesAttribute {
    fn from_raw_attr(
        raw_attr: crate::attributes::RawAttribute,
        _transaction_id: &crate::header::TransactionId,
    ) -> Result<Self, crate::errors::StunMessageError> {
        let mut attributes = Vec::with_capacity(raw_attr.value.len() / 2);
        let mut bytes = raw_attr.value.as_slice();
        while bytes.has_data_left()? {
            let attr = bytes.read_u16::<BigEndian>()?;
            if attr == 0 {
                break;
            }
            attributes.push(attr);
        }
        Ok(Self { attributes })
    }
    fn into_raw_attr(
        self,
        _transaction_id: &crate::header::TransactionId,
    ) -> crate::attributes::RawAttribute {
        let mut value = Vec::with_capacity(self.attributes.len() * 2);
        self.attributes.iter().for_each(|item| {
            value.write_u16::<BigEndian>((*item).into()).unwrap();
        });
        crate::attributes::RawAttribute::new(self.get_type(), value)
    }
}
