use std::{fmt, net::SocketAddr};

use utils::{errors::context::LocationExt, traits::dynamic_sized_packet::DynamicSizedPacket};

use crate::{
    MessageChecker,
    attributes::{
        AttributeExtDynamic, AttributeExtStatic, AttributeFactory, rfc8489::MappedAddressAttribute,
    },
    define_attribute,
    errors::StunMessageResult,
};

#[derive(Clone)]
pub struct AlternateServerAttribute(MappedAddressAttribute);

impl AlternateServerAttribute {
    pub fn new(addr: SocketAddr) -> Self {
        Self(MappedAddressAttribute::new(addr))
    }
}

impl fmt::Debug for AlternateServerAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AttrType: {:?}, [{}]:{}",
            self.get_type(),
            self.0.address(),
            self.0.port()
        )
    }
}

impl DynamicSizedPacket for AlternateServerAttribute {
    fn get_packet_bytes_count(&self) -> usize {
        self.0.get_packet_bytes_count()
    }
}

define_attribute!(0x8023, AlternateServerAttribute, "ALTERNATE_SERVER");

impl MessageChecker for AlternateServerAttribute {}

impl AttributeFactory for AlternateServerAttribute {
    fn from_raw_attr(
        raw_attr: crate::attributes::RawAttribute,
        transaction_id: &crate::header::TransactionId,
    ) -> StunMessageResult<Self> {
        Ok(Self(
            MappedAddressAttribute::from_raw_attr(raw_attr, transaction_id).trace()?,
        ))
    }
    fn into_raw_attr(
        self,
        transaction_id: &crate::header::TransactionId,
    ) -> crate::attributes::RawAttribute {
        self.0.into_raw_attr(transaction_id)
    }
}
