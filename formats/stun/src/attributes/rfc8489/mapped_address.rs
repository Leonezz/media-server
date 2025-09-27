use std::{
    fmt,
    io::Read,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use utils::traits::dynamic_sized_packet::DynamicSizedPacket;

use crate::{
    attribute::{AttrType, STUNAttributeExt, STUNRawAttribute},
    attributes::check_attr_match,
    errors::STUNMessageError,
};

///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |0 0 0 0 0 0 0 0|     Family    |               Port            |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                                                               |
/// |                 Address (32 bits or 128 bits)                 |
/// |                                                               |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Clone)]
pub struct MappedAddressAttribute {
    _family: u8,
    port: u16,
    address: IpAddr,
}

pub const ADDRESS_FAMILY_V4: u8 = 0x01;
pub const ADDRESS_FAMILY_V6: u8 = 0x02;
pub const IPV4_LEN: usize = 4;
pub const IPV6_LEN: usize = 16;

impl fmt::Debug for MappedAddressAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AttrType: {:?}, value: [{}]:{}",
            self.get_type(),
            self.address,
            self.port
        )
    }
}

impl MappedAddressAttribute {
    pub fn family(&self) -> u16 {
        match self.address {
            IpAddr::V4(_) => ADDRESS_FAMILY_V4 as u16,
            IpAddr::V6(_) => ADDRESS_FAMILY_V6 as u16,
        }
    }

    pub fn new(addr: SocketAddr) -> Self {
        match addr {
            SocketAddr::V4(v4addr) => Self {
                _family: ADDRESS_FAMILY_V4,
                port: v4addr.port(),
                address: IpAddr::V4(*v4addr.ip()),
            },
            SocketAddr::V6(v6addr) => Self {
                _family: ADDRESS_FAMILY_V6,
                port: v6addr.port(),
                address: IpAddr::V6(*v6addr.ip()),
            },
        }
    }

    pub fn address(&self) -> IpAddr {
        self.address
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

impl DynamicSizedPacket for MappedAddressAttribute {
    fn get_packet_bytes_count(&self) -> usize {
        1 + 1
            + 2
            + match self.address {
                IpAddr::V4(_) => IPV4_LEN,
                IpAddr::V6(_) => IPV6_LEN,
            }
    }
}

impl From<MappedAddressAttribute> for STUNRawAttribute {
    fn from(value: MappedAddressAttribute) -> Self {
        let mut buffer = Vec::with_capacity(value.get_packet_bytes_count());
        buffer.write_u16::<BigEndian>(value.family()).unwrap();
        buffer.write_u16::<BigEndian>(value.port).unwrap();
        buffer.extend_from_slice(value.address.as_octets());
        Self::new(AttrType::MappedAddress, buffer)
    }
}

impl TryFrom<STUNRawAttribute> for MappedAddressAttribute {
    type Error = STUNMessageError;
    fn try_from(value: STUNRawAttribute) -> Result<Self, Self::Error> {
        check_attr_match(value.attr_type, AttrType::MappedAddress)?;
        let mut bytes = value.value.as_slice();
        let first_byte = bytes.read_u8()?;
        if first_byte != 0 {
            return Err(STUNMessageError::SyntaxError(format!(
                "first byte of {:?} is not zero: {}",
                AttrType::MappedAddress,
                first_byte
            )));
        }

        let family = bytes.read_u8()?;
        let port = bytes.read_u16::<BigEndian>()?;
        let address = match family {
            ADDRESS_FAMILY_V4 => {
                let mut octets = [0_u8; IPV4_LEN];
                bytes.read_exact(&mut octets)?;
                IpAddr::V4(Ipv4Addr::from_octets(octets))
            }
            ADDRESS_FAMILY_V6 => {
                let mut octets = [0_u8; IPV6_LEN];
                bytes.read_exact(&mut octets)?;
                IpAddr::V6(Ipv6Addr::from_octets(octets))
            }
            _ => {
                return Err(STUNMessageError::SyntaxError(format!(
                    "invalid ip address family: {}",
                    family
                )));
            }
        };
        Ok(Self {
            _family: family,
            port,
            address,
        })
    }
}

impl STUNAttributeExt for MappedAddressAttribute {
    fn from_raw_attr(
        raw_attr: STUNRawAttribute,
        _transaction_id: &[u8; crate::header::TRANSACTION_ID_LEN],
    ) -> Result<Self, STUNMessageError> {
        raw_attr.try_into()
    }
    fn into_raw_attr(
        self,
        _transaction_id: &[u8; crate::header::TRANSACTION_ID_LEN],
    ) -> STUNRawAttribute {
        self.into()
    }
    fn get_type(&self) -> AttrType {
        AttrType::MappedAddress
    }
}
