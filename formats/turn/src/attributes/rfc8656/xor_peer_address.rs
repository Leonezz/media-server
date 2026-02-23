use std::{
    fmt,
    net::{IpAddr, SocketAddr},
};
use stun_formats::{
    MessageChecker,
    attributes::{AttributeExtDynamic, AttributeExtStatic, AttributeFactory, check_attr_match},
    define_attribute,
};

// The XOR-PEER-ADDRESS attribute specifies the address and port
// of the peer as seen from the TURN server.
// (For example, the peer's server-reflexive transport address
// if the peer is behind a NAT.) It is encoded in the same way as the XOR-MAPPED-ADDRESS attribute.
#[derive(Clone)]
pub struct XorPeerAddressAttribute(stun_formats::attributes::rfc8489::XorMappedAddressAttribute);

impl XorPeerAddressAttribute {
    pub fn new(addr: SocketAddr) -> Self {
        Self(
            stun_formats::attributes::rfc8489::XorMappedAddressAttribute::new(SocketAddr::new(
                addr.ip(),
                addr.port(),
            )),
        )
    }

    pub fn ip(&self) -> IpAddr {
        self.0.ip()
    }

    pub fn address(&self) -> SocketAddr {
        self.0.address()
    }
}

impl fmt::Display for XorPeerAddressAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "xor peer address: [{}]:{}", self.0.ip(), self.0.port())
    }
}
impl fmt::Debug for XorPeerAddressAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

define_attribute!(0x0012, XorPeerAddressAttribute, "XOR_PEER_ADDRESS");

impl MessageChecker for XorPeerAddressAttribute {}

impl AttributeFactory for XorPeerAddressAttribute {
    fn from_raw_attr(
        raw_attr: stun_formats::attributes::RawAttribute,
        transaction_id: &stun_formats::header::TransactionId,
    ) -> Result<Self, stun_formats::errors::StunMessageError> {
        check_attr_match(raw_attr.attr_type, Self::STATIC_ATTR_TYPE)?;
        let mut buffer = raw_attr.value.as_slice();
        let address =
            stun_formats::attributes::rfc8489::XorMappedAddressAttribute::read_without_type(
                &mut buffer,
                transaction_id,
            )?;
        Ok(Self(address))
    }

    fn into_raw_attr(
        self,
        transaction_id: &stun_formats::header::TransactionId,
    ) -> stun_formats::attributes::RawAttribute {
        let mut value = Vec::new();
        self.0
            .write_without_type(&mut value, transaction_id)
            .unwrap();
        stun_formats::attributes::RawAttribute::new(Self::STATIC_ATTR_TYPE, value)
    }
}
