use std::{fmt, io};

use crate::{
    MessageChecker,
    attributes::{
        AttributeExtDynamic, AttributeExtStatic, AttributeFactory, STUN_ATTRIBUTE_PADDING_SIZE,
        check_attr_match, get_after_padding_size,
    },
    define_attribute,
    errors::{StunMessageError, StunMessageResult},
    header::MessageClass,
};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use rootcause::bail;
use utils::{
    errors::context::LocationExt,
    traits::{dynamic_sized_packet::DynamicSizedPacket, reader::ReadFrom, writer::WriteTo},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Algorithm(u16);

pub const ALGORITHM_MD5: Algorithm = Algorithm(0x0001);
pub const ALGORITHM_SHA256: Algorithm = Algorithm(0x0002);

impl fmt::Debug for Algorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match *self {
            ALGORITHM_MD5 => "MD5",
            ALGORITHM_SHA256 => "SHA256",
            _ => return write!(f, "Reserved(0x{:x})", self.0),
        };
        f.write_str(str)
    }
}

///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// | Algorithm                     |  Algorithm Parameters Length  |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// | Algorithm Parameters (variable)
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Clone)]
pub struct PasswordAlgorithm {
    pub algorithm: Algorithm,
    pub parameters: Vec<u8>,
}

impl fmt::Debug for PasswordAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "algorithm: {:?}, parameters: 0x{:x?}",
            self.algorithm, self.parameters
        )
    }
}

impl DynamicSizedPacket for PasswordAlgorithm {
    fn get_packet_bytes_count(&self) -> usize {
        get_after_padding_size(2 + 2 + self.parameters.len(), STUN_ATTRIBUTE_PADDING_SIZE)
    }
}

impl<R: io::Read> ReadFrom<R> for PasswordAlgorithm {
    type Error = StunMessageError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let algorithm = reader.read_u16::<BigEndian>()?;
        let length = reader.read_u16::<BigEndian>()?;
        let parameter_len = get_after_padding_size(length as usize, STUN_ATTRIBUTE_PADDING_SIZE);
        let mut parameters = vec![0_u8; parameter_len];
        reader.read_exact(&mut parameters)?;
        Ok(Self {
            algorithm: Algorithm(algorithm),
            parameters: parameters[..length as usize].into(),
        })
    }
}

impl<W: io::Write> WriteTo<W> for PasswordAlgorithm {
    type Error = StunMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u16::<BigEndian>(self.algorithm.0)?;
        writer.write_u16::<BigEndian>(self.parameters.len() as u16)?;
        writer.write_all(&self.parameters)?;
        let padding_size =
            get_after_padding_size(self.parameters.len(), STUN_ATTRIBUTE_PADDING_SIZE);
        writer.write_all(&vec![0; padding_size])?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct PasswordAlgorithmAttribute(PasswordAlgorithm);
impl DynamicSizedPacket for PasswordAlgorithmAttribute {
    fn get_packet_bytes_count(&self) -> usize {
        self.0.get_packet_bytes_count()
    }
}

impl fmt::Debug for PasswordAlgorithmAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AttrType: {:?}, {:?}", self.get_type(), self.0)
    }
}

define_attribute!(0x001D, PasswordAlgorithmAttribute, "PASSWORD_ALGORITHM");

impl MessageChecker for PasswordAlgorithmAttribute {
    fn check(&self, message: &crate::message::Message) -> crate::errors::StunMessageResult<()> {
        let class = message.message_class();
        if !matches!(class, MessageClass::Request) {
            bail!(StunMessageError::InvalidMessage(format!(
                "{:?} in {:?} message is not allowed",
                Self::STATIC_ATTR_TYPE,
                class
            )))
        }
        Ok(())
    }
}

impl AttributeFactory for PasswordAlgorithmAttribute {
    fn from_raw_attr(
        raw_attr: crate::attributes::RawAttribute,
        _transaction_id: &crate::header::TransactionId,
    ) -> StunMessageResult<Self> {
        check_attr_match(raw_attr.attr_type, Self::STATIC_ATTR_TYPE).trace()?;
        let algorithm = PasswordAlgorithm::read_from(&mut raw_attr.value.as_slice())?;
        Ok(Self(algorithm))
    }
    fn into_raw_attr(
        self,
        _transaction_id: &crate::header::TransactionId,
    ) -> crate::attributes::RawAttribute {
        let mut value = Vec::with_capacity(self.0.get_packet_bytes_count());
        self.0.write_to(&mut value).unwrap();
        crate::attributes::RawAttribute::new(self.get_type(), value)
    }
}
