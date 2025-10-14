use std::{fmt, io::BufRead};

use utils::traits::{dynamic_sized_packet::DynamicSizedPacket, reader::ReadFrom, writer::WriteTo};

use crate::{
    MessageChecker,
    attributes::{
        AttributeExtDynamic, AttributeExtStatic, AttributeFactory, check_attr_match,
        rfc8489::password_algorithm::PasswordAlgorithm,
    },
    define_attribute,
};

///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// | Algorithm 1                   | Algorithm 1 Parameters Length |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// | Algorithm 1 Parameters (variable)
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// | Algorithm 2                   | Algorithm 2 Parameters Length |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// | Algorithm 2 Parameters (variable)
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                                                             ...
#[derive(Clone)]
pub struct PasswordAlgorithmsAttribute {
    pub algorithms: Vec<PasswordAlgorithm>,
}

impl fmt::Debug for PasswordAlgorithmsAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AttrType: {:?}, value: {:?}",
            self.get_type(),
            self.algorithms
        )
    }
}

impl DynamicSizedPacket for PasswordAlgorithmsAttribute {
    fn get_packet_bytes_count(&self) -> usize {
        self.algorithms
            .iter()
            .fold(0, |prev, item| prev + item.get_packet_bytes_count())
    }
}

define_attribute!(0x8002, PasswordAlgorithmsAttribute, "PASSWORD_ALGORITHMS");

impl MessageChecker for PasswordAlgorithmsAttribute {}

impl AttributeFactory for PasswordAlgorithmsAttribute {
    fn from_raw_attr(
        raw_attr: crate::attributes::RawAttribute,
        _transaction_id: &crate::header::TransactionId,
    ) -> Result<Self, crate::errors::StunMessageError> {
        check_attr_match(raw_attr.attr_type, Self::STATIC_ATTR_TYPE)?;
        let mut bytes = raw_attr.value.as_slice();
        let mut algorithms = Vec::new();
        while bytes.has_data_left()? {
            algorithms.push(PasswordAlgorithm::read_from(&mut bytes)?);
        }
        Ok(Self { algorithms })
    }
    fn into_raw_attr(
        self,
        _transaction_id: &crate::header::TransactionId,
    ) -> crate::attributes::RawAttribute {
        let mut value = Vec::with_capacity(self.get_packet_bytes_count());
        self.algorithms.iter().for_each(|item| {
            item.write_to(&mut value).unwrap();
        });
        crate::attributes::RawAttribute::new(self.get_type(), value)
    }
}
