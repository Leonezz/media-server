use std::{
    fmt, io,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use utils::traits::{dynamic_sized_packet::DynamicSizedPacket, writer::WriteTo};

use crate::{
    MessageChecker,
    attributes::{
        AttributeExtDynamic, AttributeExtStatic, AttributeFactory, check_attr_match,
        rfc8489::mapped_address::{ADDRESS_FAMILY_V4, ADDRESS_FAMILY_V6, IPV4_LEN, IPV6_LEN},
    },
    define_attribute,
    errors::StunMessageResult,
    header::{MAGIC_COOKIE, TRANSACTION_ID_LEN},
};

///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |0 0 0 0 0 0 0 0|     Family    |            X-Port             |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// | X-Address (Variable)
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Clone)]
pub struct XorMappedAddressAttribute {
    _family: u8,
    xport: u16,
    xaddress: IpAddr,
}

impl fmt::Debug for XorMappedAddressAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AttrType: {:?}, value: [{}]:{}",
            self.get_type(),
            self.xaddress,
            self.xport
        )
    }
}

impl DynamicSizedPacket for XorMappedAddressAttribute {
    fn get_packet_bytes_count(&self) -> usize {
        1 + 1
            + 2
            + match self.xaddress {
                IpAddr::V4(_) => IPV4_LEN,
                IpAddr::V6(_) => IPV6_LEN,
            }
    }
}

impl XorMappedAddressAttribute {
    pub fn family(&self) -> u16 {
        match self.xaddress {
            IpAddr::V4(_) => ADDRESS_FAMILY_V4 as u16,
            IpAddr::V6(_) => ADDRESS_FAMILY_V6 as u16,
        }
    }

    pub fn new(addr: SocketAddr) -> Self {
        match addr {
            SocketAddr::V4(v4addr) => Self {
                _family: ADDRESS_FAMILY_V4,
                xport: v4addr.port(),
                xaddress: IpAddr::V4(*v4addr.ip()),
            },
            SocketAddr::V6(v6addr) => Self {
                _family: ADDRESS_FAMILY_V6,
                xport: v6addr.port(),
                xaddress: IpAddr::V6(*v6addr.ip()),
            },
        }
    }

    pub fn port(&self) -> u16 {
        self.xport
    }

    pub fn ip(&self) -> IpAddr {
        self.xaddress
    }

    pub fn address(&self) -> SocketAddr {
        SocketAddr::new(self.ip(), self.port())
    }

    pub fn read_without_type<R: io::Read>(
        reader: &mut R,
        transaction_id: &crate::header::TransactionId,
    ) -> StunMessageResult<Self> {
        let first_byte = reader.read_u8()?;
        if first_byte != 0 {
            return Err(crate::errors::StunMessageError::SyntaxError(format!(
                "first byte of {:?} is not 0: {}",
                Self::STATIC_ATTR_TYPE,
                first_byte
            )));
        }
        let family = reader.read_u8()?;
        let port = reader.read_u16::<BigEndian>()?;
        let port = port ^ (MAGIC_COOKIE >> 16) as u16;
        let address = match family {
            ADDRESS_FAMILY_V4 => {
                let mut ipv4_bytes = [0_u8; IPV4_LEN];
                reader.read_exact(&mut ipv4_bytes)?;
                xor_inplace(&mut ipv4_bytes, &MAGIC_COOKIE.to_be_bytes());
                IpAddr::V4(Ipv4Addr::from_octets(ipv4_bytes))
            }
            ADDRESS_FAMILY_V6 => {
                let mut ipv6_bytes = [0_u8; IPV6_LEN];
                reader.read_exact(&mut ipv6_bytes)?;
                let mut xor_value = Vec::with_capacity(4 + TRANSACTION_ID_LEN);
                xor_value.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());
                transaction_id.write_to(&mut xor_value).unwrap();
                xor_inplace(&mut ipv6_bytes, &xor_value.try_into().unwrap());
                IpAddr::V6(Ipv6Addr::from_octets(ipv6_bytes))
            }
            _ => {
                return Err(crate::errors::StunMessageError::SyntaxError(format!(
                    "invalid ip address family: {}",
                    family
                )));
            }
        };
        Ok(Self {
            _family: family,
            xport: port,
            xaddress: address,
        })
    }

    pub fn write_without_type<W: io::Write>(
        &self,
        writer: &mut W,
        transaction_id: &crate::header::TransactionId,
    ) -> StunMessageResult<()> {
        writer.write_u16::<BigEndian>(self.family())?;
        writer.write_u16::<BigEndian>(self.xport ^ (MAGIC_COOKIE >> 16) as u16)?;
        match self.xaddress {
            IpAddr::V4(ipv4) => {
                let mut ipv4_bytes = ipv4.octets();
                xor_inplace(&mut ipv4_bytes, &MAGIC_COOKIE.to_be_bytes());
                writer.write_all(&ipv4_bytes)?;
            }
            IpAddr::V6(ipv6) => {
                let mut ipv6_bytes = ipv6.octets();
                let mut xor_value = Vec::with_capacity(4 + TRANSACTION_ID_LEN);
                xor_value.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());
                transaction_id.write_to(&mut xor_value).unwrap();
                xor_inplace(&mut ipv6_bytes, &xor_value.try_into().unwrap());
                writer.write_all(&ipv6_bytes)?;
            }
        }
        Ok(())
    }
}

fn xor_inplace<const L: usize>(dst: &mut [u8; L], xor: &[u8; L]) {
    for (x, y) in dst.iter_mut().zip(xor.iter()) {
        *x ^= *y;
    }
}

define_attribute!(0x0020, XorMappedAddressAttribute, "XOR_MAPPED_ADDRESS");

impl MessageChecker for XorMappedAddressAttribute {}

impl AttributeFactory for XorMappedAddressAttribute {
    fn from_raw_attr(
        raw_attr: crate::attributes::RawAttribute,
        transaction_id: &crate::header::TransactionId,
    ) -> Result<Self, crate::errors::StunMessageError> {
        check_attr_match(raw_attr.attr_type, Self::STATIC_ATTR_TYPE)?;
        let mut bytes = raw_attr.value.as_slice();
        Self::read_without_type(&mut bytes, transaction_id)
    }
    fn into_raw_attr(
        self,
        transaction_id: &crate::header::TransactionId,
    ) -> crate::attributes::RawAttribute {
        let mut value = Vec::with_capacity(self.get_packet_bytes_count());
        self.write_without_type(&mut value, transaction_id).unwrap();
        crate::attributes::RawAttribute::new(self.get_type(), value)
    }
}
