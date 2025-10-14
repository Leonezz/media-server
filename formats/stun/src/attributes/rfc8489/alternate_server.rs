use std::{fmt, net::SocketAddr};

use utils::traits::dynamic_sized_packet::DynamicSizedPacket;

use crate::{attribute::AttributeExt, rfc8489::MappedAddressAttribute};

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

impl AttributeExt for AlternateServerAttribute {
    const STATIC_ATTR_TYPE: Option<crate::attribute::AttrType> =
        Some(crate::attribute::AttrType::AlternateServer);
    fn get_type(&self) -> crate::attribute::AttrType {
        Self::STATIC_ATTR_TYPE.unwrap()
    }

    fn from_raw_attr(
        raw_attr: crate::attribute::RawAttribute,
        transaction_id: &crate::header::TransactionId,
    ) -> Result<Self, crate::errors::StunMessageError> {
        Ok(Self(MappedAddressAttribute::from_raw_attr(
            raw_attr,
            transaction_id,
        )?))
    }

    fn into_raw_attr(
        self,
        transaction_id: &crate::header::TransactionId,
    ) -> crate::attribute::RawAttribute {
        self.0.into_raw_attr(transaction_id)
    }
}
