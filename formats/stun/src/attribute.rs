use crate::{
    attributes::{STUN_ATTRIBUTE_PADDING_SIZE, get_after_padding_size},
    errors::StunMessageError,
    header::TransactionId,
    rfc8489::{self, FingerPrintAttribute, UserHashAttribute},
};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use num::ToPrimitive;
use std::{fmt, io};
use utils::traits::{
    dynamic_sized_packet::DynamicSizedPacket,
    fixed_packet::FixedPacket,
    reader::{ReadFrom, ReadRemainingFrom},
    writer::WriteTo,
};

///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |             Type              |            Length             |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                        Value (variable)                    ....
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum AttrType {
    MappedAddress = 0x0001,
    UserName = 0x0006,
    MessageIntegrity = 0x0008,
    ErrorCode = 0x0009,
    UnknownAttributes = 0x000A,
    Realm = 0x0014,
    Nonce = 0x0015,
    XorMappedAddress = 0x0020,

    MessageIntegritySHA256 = 0x001C,
    PasswordAlgorithm = 0x001D,
    UserHash = 0x001E,

    Software = 0x8022,
    AlternateServer = 0x8023,
    FingerPrint = 0x8028,

    PasswordAlgorithms = 0x8002,
    AlternateDomain = 0x8003,

    Reserved(u16),
}

impl fmt::Debug for AttrType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            Self::MappedAddress => "MAPPED_ADDRESS",
            Self::UserName => "USER_NAME",
            Self::MessageIntegrity => "MESSAGE_INTEGRITY",
            Self::ErrorCode => "ERROR_CODE",
            Self::UnknownAttributes => "UNKNOWN_ATTRIBUTES",
            Self::Realm => "REALM",
            Self::Nonce => "NONCE",
            Self::XorMappedAddress => "XOR_MAPPED_ADDRESS",
            Self::MessageIntegritySHA256 => "MESSAGE_INTEGRITY_SHA256",
            Self::PasswordAlgorithm => "PASSWORD_ALGORITHM",
            Self::UserHash => "USER_HASH",
            Self::Software => "SOFTWARE",
            Self::AlternateServer => "ALTERNATE_SERVER",
            Self::FingerPrint => "FINGER_PRINT",
            Self::PasswordAlgorithms => "PASSWORD_ALGORITHMS",
            Self::AlternateDomain => "ALTERNATE_DOMAIN",
            Self::Reserved(v) => return write!(f, "Reserved(0x{:x})", v),
        };
        f.write_str(str)
    }
}

impl From<AttrType> for u16 {
    fn from(value: AttrType) -> Self {
        match value {
            AttrType::MappedAddress => 0x0001,
            AttrType::UserName => 0x0006,
            AttrType::MessageIntegrity => 0x0008,
            AttrType::ErrorCode => 0x0009,
            AttrType::UnknownAttributes => 0x000A,
            AttrType::Realm => 0x0014,
            AttrType::Nonce => 0x0015,
            AttrType::XorMappedAddress => 0x0020,
            AttrType::MessageIntegritySHA256 => 0x001C,
            AttrType::PasswordAlgorithm => 0x001D,
            AttrType::UserHash => 0x001E,
            AttrType::Software => 0x8022,
            AttrType::AlternateServer => 0x8023,
            AttrType::FingerPrint => 0x8028,
            AttrType::PasswordAlgorithms => 0x8002,
            AttrType::AlternateDomain => 0x8003,
            AttrType::Reserved(v) => v,
        }
    }
}

impl From<u16> for AttrType {
    fn from(value: u16) -> Self {
        match value {
            0x0001 => Self::MappedAddress,
            0x0006 => Self::UserName,
            0x0008 => Self::MessageIntegrity,
            0x0009 => Self::ErrorCode,
            0x000A => Self::UnknownAttributes,
            0x0014 => Self::Realm,
            0x0015 => Self::Nonce,
            0x0020 => Self::XorMappedAddress,
            0x001C => Self::MessageIntegritySHA256,
            0x001D => Self::PasswordAlgorithm,
            0x001E => Self::UserHash,
            0x8022 => Self::Software,
            0x8023 => Self::AlternateServer,
            0x8028 => Self::FingerPrint,
            0x8002 => Self::PasswordAlgorithms,
            0x8003 => Self::AlternateDomain,
            v => Self::Reserved(v),
        }
    }
}

#[derive(Clone)]
pub struct RawAttribute {
    pub attr_type: AttrType,
    length: u16,
    pub value: Vec<u8>,
}

impl AttributeExt for RawAttribute {
    fn get_type(&self) -> AttrType {
        self.attr_type
    }

    fn from_raw_attr(
        raw_attr: RawAttribute,
        _transaction_id: &TransactionId,
    ) -> Result<Self, StunMessageError> {
        Ok(raw_attr)
    }

    fn into_raw_attr(self, _transaction_id: &TransactionId) -> RawAttribute {
        self
    }
}

impl fmt::Debug for RawAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AttrType: {:?}, value: 0x{:x?}",
            self.attr_type, self.value
        )
    }
}

impl RawAttribute {
    pub fn new(attr_type: AttrType, value: Vec<u8>) -> Self {
        assert_eq!(value.len().to_u16(), Some(value.len() as u16));
        Self {
            attr_type,
            length: value.len() as u16,
            value,
        }
    }
}

pub trait AttributeExt: Sized {
    fn from_raw_attr(
        raw_attr: RawAttribute,
        transaction_id: &TransactionId,
    ) -> Result<Self, StunMessageError>;
    fn into_raw_attr(self, transaction_id: &TransactionId) -> RawAttribute;
    fn get_type(&self) -> AttrType;
    fn is_comprehension_required(&self) -> bool {
        u16::from(self.get_type()) < 0x8000
    }
}

impl<R: io::Read> ReadFrom<R> for RawAttribute {
    type Error = StunMessageError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let attr_type = reader.read_u16::<BigEndian>()?.into();
        Self::read_remaining_from(attr_type, reader)
    }
}

impl<R: io::Read> ReadRemainingFrom<AttrType, R> for RawAttribute {
    type Error = StunMessageError;
    fn read_remaining_from(header: AttrType, reader: &mut R) -> Result<Self, Self::Error> {
        let length = reader.read_u16::<BigEndian>()?;
        let mut value = vec![0_u8; length as usize];
        reader.read_exact(&mut value)?;
        let padding_bytes_cnt =
            get_after_padding_size(length as usize, STUN_ATTRIBUTE_PADDING_SIZE) - length as usize;
        for _ in 0..padding_bytes_cnt {
            let _ = reader.read_u8()?;
        }
        Ok(Self {
            attr_type: header,
            length,
            value,
        })
    }
}

impl<W: io::Write> WriteTo<W> for RawAttribute {
    type Error = StunMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        debug_assert_eq!(self.value.len().to_u16(), Some(self.length));
        writer.write_u16::<BigEndian>(self.attr_type.into())?;
        writer.write_u16::<BigEndian>(self.value.len().to_u16().unwrap())?;
        writer.write_all(&self.value)?;
        let padding_bytes_len =
            get_after_padding_size(self.value.len(), STUN_ATTRIBUTE_PADDING_SIZE)
                - self.value.len();
        for _ in 0..padding_bytes_len {
            writer.write_u8(0)?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub enum Attribute {
    MappedAddress(rfc8489::MappedAddressAttribute),
    UserName(rfc8489::UserNameAttribute),
    MessageIntegrity(rfc8489::MessageIntegrityAttribute),
    ErrorCode(rfc8489::ErrorCodeAttribute),
    UnknownAttributes(rfc8489::UnknownAttributes),
    Realm(rfc8489::RealmAttribute),
    Nonce(rfc8489::NonceAttribute),
    XorMappedAddress(rfc8489::XorMappedAddressAttribute),
    MessageIntegritySHA256(rfc8489::MessageIntegritySHA256Attribute),
    PasswordAlgorithm(rfc8489::PasswordAlgorithmAttribute),
    UserHash(rfc8489::UserHashAttribute),
    Software(rfc8489::SoftwareAttribute),
    AlternateServer(rfc8489::AlternateServerAttribute),
    FingerPrint(rfc8489::FingerPrintAttribute),
    PasswordAlgorithms(rfc8489::PasswordAlgorithmsAttribute),
    AlternateDomain(rfc8489::AlternateDomainAttribute),

    Raw(RawAttribute),
}

impl fmt::Debug for Attribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MappedAddress(s) => write!(f, "{:?}", s),
            Self::UserName(s) => write!(f, "{:?}", s),
            Self::MessageIntegrity(s) => write!(f, "{:?}", s),
            Self::ErrorCode(s) => write!(f, "{:?}", s),
            Self::UnknownAttributes(s) => write!(f, "{:?}", s),
            Self::Realm(s) => write!(f, "{:?}", s),
            Self::Nonce(s) => write!(f, "{:?}", s),
            Self::XorMappedAddress(s) => write!(f, "{:?}", s),
            Self::MessageIntegritySHA256(s) => write!(f, "{:?}", s),
            Self::PasswordAlgorithm(s) => write!(f, "{:?}", s),
            Self::UserHash(s) => write!(f, "{:?}", s),
            Self::Software(s) => write!(f, "{:?}", s),
            Self::AlternateServer(s) => write!(f, "{:?}", s),
            Self::FingerPrint(s) => write!(f, "{:?}", s),
            Self::PasswordAlgorithms(s) => write!(f, "{:?}", s),
            Self::AlternateDomain(s) => write!(f, "{:?}", s),
            Self::Raw(s) => write!(f, "{:?}", s),
        }
    }
}

impl AttributeExt for Attribute {
    fn get_type(&self) -> AttrType {
        match self {
            Self::MappedAddress(s) => s.get_type(),
            Self::UserName(s) => s.get_type(),
            Self::MessageIntegrity(s) => s.get_type(),
            Self::ErrorCode(s) => s.get_type(),
            Self::UnknownAttributes(s) => s.get_type(),
            Self::Realm(s) => s.get_type(),
            Self::Nonce(s) => s.get_type(),
            Self::XorMappedAddress(s) => s.get_type(),
            Self::MessageIntegritySHA256(s) => s.get_type(),
            Self::PasswordAlgorithm(s) => s.get_type(),
            Self::UserHash(s) => s.get_type(),
            Self::Software(s) => s.get_type(),
            Self::AlternateServer(s) => s.get_type(),
            Self::FingerPrint(s) => s.get_type(),
            Self::PasswordAlgorithms(s) => s.get_type(),
            Self::AlternateDomain(s) => s.get_type(),
            Self::Raw(s) => s.attr_type,
        }
    }

    fn from_raw_attr(
        raw_attr: RawAttribute,
        transaction_id: &TransactionId,
    ) -> Result<Self, StunMessageError> {
        let attr = match raw_attr.attr_type {
            AttrType::MappedAddress => Attribute::MappedAddress(
                rfc8489::MappedAddressAttribute::from_raw_attr(raw_attr, transaction_id)?,
            ),
            AttrType::UserName => Attribute::UserName(rfc8489::UserNameAttribute::from_raw_attr(
                raw_attr,
                transaction_id,
            )?),
            AttrType::MessageIntegrity => Attribute::MessageIntegrity(
                rfc8489::MessageIntegrityAttribute::from_raw_attr(raw_attr, transaction_id)?,
            ),
            AttrType::ErrorCode => Attribute::ErrorCode(
                rfc8489::ErrorCodeAttribute::from_raw_attr(raw_attr, transaction_id)?,
            ),
            AttrType::UnknownAttributes => Attribute::UnknownAttributes(
                rfc8489::UnknownAttributes::from_raw_attr(raw_attr, transaction_id)?,
            ),
            AttrType::Realm => Attribute::Realm(rfc8489::RealmAttribute::from_raw_attr(
                raw_attr,
                transaction_id,
            )?),
            AttrType::Nonce => Attribute::Nonce(rfc8489::NonceAttribute::from_raw_attr(
                raw_attr,
                transaction_id,
            )?),
            AttrType::XorMappedAddress => Attribute::XorMappedAddress(
                rfc8489::XorMappedAddressAttribute::from_raw_attr(raw_attr, transaction_id)?,
            ),
            AttrType::MessageIntegritySHA256 => Attribute::MessageIntegritySHA256(
                rfc8489::MessageIntegritySHA256Attribute::from_raw_attr(raw_attr, transaction_id)?,
            ),
            AttrType::PasswordAlgorithm => Attribute::PasswordAlgorithm(
                rfc8489::PasswordAlgorithmAttribute::from_raw_attr(raw_attr, transaction_id)?,
            ),
            AttrType::UserHash => Attribute::UserHash(rfc8489::UserHashAttribute::from_raw_attr(
                raw_attr,
                transaction_id,
            )?),
            AttrType::Software => Attribute::Software(rfc8489::SoftwareAttribute::from_raw_attr(
                raw_attr,
                transaction_id,
            )?),
            AttrType::AlternateServer => Attribute::AlternateServer(
                rfc8489::AlternateServerAttribute::from_raw_attr(raw_attr, transaction_id)?,
            ),
            AttrType::FingerPrint => Attribute::FingerPrint(
                rfc8489::FingerPrintAttribute::from_raw_attr(raw_attr, transaction_id)?,
            ),
            AttrType::PasswordAlgorithms => Attribute::PasswordAlgorithms(
                rfc8489::PasswordAlgorithmsAttribute::from_raw_attr(raw_attr, transaction_id)?,
            ),
            AttrType::AlternateDomain => Attribute::AlternateDomain(
                rfc8489::AlternateDomainAttribute::from_raw_attr(raw_attr, transaction_id)?,
            ),
            AttrType::Reserved(_) => Attribute::Raw(raw_attr),
        };
        Ok(attr)
    }

    fn into_raw_attr(self, transaction_id: &TransactionId) -> RawAttribute {
        match self {
            Self::MappedAddress(s) => s.into_raw_attr(transaction_id),
            Self::UserName(s) => s.into_raw_attr(transaction_id),
            Self::MessageIntegrity(s) => s.into_raw_attr(transaction_id),
            Self::ErrorCode(s) => s.into_raw_attr(transaction_id),
            Self::UnknownAttributes(s) => s.into_raw_attr(transaction_id),
            Self::Realm(s) => s.into_raw_attr(transaction_id),
            Self::Nonce(s) => s.into_raw_attr(transaction_id),
            Self::XorMappedAddress(s) => s.into_raw_attr(transaction_id),
            Self::MessageIntegritySHA256(s) => s.into_raw_attr(transaction_id),
            Self::PasswordAlgorithm(s) => s.into_raw_attr(transaction_id),
            Self::UserHash(s) => s.into_raw_attr(transaction_id),
            Self::Software(s) => s.into_raw_attr(transaction_id),
            Self::AlternateServer(s) => s.into_raw_attr(transaction_id),
            Self::FingerPrint(s) => s.into_raw_attr(transaction_id),
            Self::PasswordAlgorithms(s) => s.into_raw_attr(transaction_id),
            Self::AlternateDomain(s) => s.into_raw_attr(transaction_id),
            Self::Raw(s) => s.into_raw_attr(transaction_id),
        }
    }
}

impl DynamicSizedPacket for Attribute {
    fn get_packet_bytes_count(&self) -> usize {
        2 + 2
            + match self {
                Self::MappedAddress(s) => s.get_packet_bytes_count(),
                Self::UserName(s) => s.get_packet_bytes_count(),
                Self::MessageIntegrity(s) => s.get_packet_bytes_count(),
                Self::ErrorCode(s) => s.get_packet_bytes_count(),
                Self::UnknownAttributes(s) => s.get_packet_bytes_count(),
                Self::Realm(s) => s.get_packet_bytes_count(),
                Self::Nonce(s) => s.get_packet_bytes_count(),
                Self::XorMappedAddress(s) => s.get_packet_bytes_count(),
                Self::MessageIntegritySHA256(s) => s.get_packet_bytes_count(),
                Self::PasswordAlgorithm(s) => s.get_packet_bytes_count(),
                Self::UserHash(_) => UserHashAttribute::bytes_count(),
                Self::Software(s) => s.get_packet_bytes_count(),
                Self::AlternateServer(s) => s.get_packet_bytes_count(),
                Self::FingerPrint(_) => FingerPrintAttribute::bytes_count(),
                Self::PasswordAlgorithms(s) => s.get_packet_bytes_count(),
                Self::AlternateDomain(s) => s.get_packet_bytes_count(),
                Self::Raw(s) => s.value.len(),
            }
    }
}
