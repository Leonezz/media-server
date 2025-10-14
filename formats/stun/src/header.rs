use crate::{
    attributes::rfc8489,
    errors::{StunMessageError, StunMessageResult},
    methods::{MethodExtDynamic, MethodExtStatic, from_value},
};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::{fmt, io};
use utils::traits::{fixed_packet::FixedPacket, reader::ReadFrom, writer::WriteTo};

// see: Section 5. STUN Message Structure
//  0                   1                   2                   3
//  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |0 0|     STUN Message Type     |         Message Length        |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                          Magic Cookie                         |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                                                               |
// |                   Transaction ID (96 bits)                    |
// |                                                               |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

//  0                 1
//  2   3 4 5 6 7 8 9 0 1 2 3 4 5
// +--+--+-+-+-+-+-+-+-+-+-+-+-+-+
// |M |M |M|M|M|C|M|M|M|C|M|M|M|M|
// |11|10|9|8|7|1|6|5|4|0|3|2|1|0|
// +--+--+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MessageClass {
    Request,
    Indication,
    SuccessResponse,
    ErrorResponse,
}

impl fmt::Debug for MessageClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            Self::Request => "request",
            Self::Indication => "indication",
            Self::SuccessResponse => "success response",
            Self::ErrorResponse => "error response",
        };
        f.write_str(str)
    }
}

impl From<u8> for MessageClass {
    fn from(value: u8) -> Self {
        match value & 0b11 {
            0b00 => Self::Request,
            0b01 => Self::Indication,
            0b10 => Self::SuccessResponse,
            0b11 => Self::ErrorResponse,
            _ => unreachable!(),
        }
    }
}

impl From<MessageClass> for u8 {
    fn from(value: MessageClass) -> Self {
        match value {
            MessageClass::Request => 0b00,
            MessageClass::Indication => 0b01,
            MessageClass::SuccessResponse => 0b10,
            MessageClass::ErrorResponse => 0b11,
        }
    }
}

impl MessageClass {
    pub fn c1c0(&self) -> u8 {
        u8::from(*self)
    }

    pub fn c0(&self) -> bool {
        (self.c1c0() & 0b1) == 0b1
    }

    pub fn c1(&self) -> bool {
        ((self.c1c0() >> 1) & 0b1) == 0b1
    }

    pub fn new(c1: bool, c0: bool) -> Self {
        let value = ((c1 as u8) << 1) | (c0 as u8);
        Self::from(value)
    }

    pub fn check(&self, message: &crate::message::Message) -> StunMessageResult<()> {
        match self {
            Self::ErrorResponse => message.require_ext::<rfc8489::ErrorCodeAttribute>(),
            Self::Indication => Ok(()),
            Self::Request => Ok(()),
            Self::SuccessResponse => Ok(()),
        }
    }
}

pub struct MessageType {
    pub method: Box<dyn MethodExtDynamic>, // 12 bits
    pub message_class: MessageClass,
}

impl Clone for MessageType {
    fn clone(&self) -> Self {
        Self {
            method: from_value(self.method.value()).unwrap(),
            message_class: self.message_class,
        }
    }
}

impl fmt::Debug for MessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {:?}", self.message_class, self.method)
    }
}

impl MessageType {
    pub fn new<M: MethodExtStatic>(class: MessageClass) -> Self {
        Self::new_method(from_value(M::STATIC_VALUE).unwrap(), class)
    }

    pub fn new_method(method: Box<dyn MethodExtDynamic>, class: MessageClass) -> Self {
        Self {
            method,
            message_class: class,
        }
    }
}

impl TryFrom<u16> for MessageType {
    type Error = StunMessageError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        let c0 = ((value >> 4) & 0b1) == 0b1;
        let c1 = ((value >> 8) & 0b1) == 0b1;
        let m0_3 = value & 0b1111;
        let m4_6 = (value >> 5) & 0b111;
        let m7_11 = (value >> 9) & 0b11111;
        let method = from_value(m0_3 | (m4_6 << 4) | (m7_11 << 7));
        if method.is_none() {
            return Err(StunMessageError::UnknownMethod(value));
        }
        Ok(Self {
            method: method.unwrap(),
            message_class: MessageClass::new(c1, c0),
        })
    }
}

impl From<&MessageType> for u16 {
    fn from(value: &MessageType) -> Self {
        let method_value: u16 = value.method.value();
        let m0_3 = method_value & 0b1111;
        let m4_6 = (method_value >> 4) & 0b111;
        let m7_11 = (method_value >> 7) & 0b11111;
        let c0 = value.message_class.c0() as u16;
        let c1 = value.message_class.c1() as u16;
        m0_3 | (c0 << 4) | (m4_6 << 5) | (c1 << 8) | (m7_11 << 9)
    }
}

pub const MAGIC_COOKIE: u32 = 0x2112A442;
pub const TRANSACTION_ID_LEN: usize = 12;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransactionId([u8; TRANSACTION_ID_LEN]);

impl fmt::Debug for TransactionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x")?;
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

impl TransactionId {
    pub fn new(id: [u8; TRANSACTION_ID_LEN]) -> Self {
        Self(id)
    }

    pub fn new_random() -> Self {
        let mut id = [0_u8; TRANSACTION_ID_LEN];
        utils::random::random_fill(&mut id);
        Self(id)
    }

    pub fn new_dummy() -> Self {
        Self(Default::default())
    }
}

impl<R: io::Read> ReadFrom<R> for TransactionId {
    type Error = StunMessageError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let mut value = Self::new_dummy();
        reader.read_exact(&mut value.0)?;
        Ok(value)
    }
}

impl<W: io::Write> WriteTo<W> for TransactionId {
    type Error = StunMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_all(&self.0)?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct MessageHeader {
    #[allow(unused)]
    reserved_zero_2_bits: u8, // 2 bits, must be 0
    pub stun_message_type: MessageType, // 14 bits
    /// The message length MUST contain the size of the message in bytes,
    /// not including the 20-byte STUN header.
    /// Since all STUN attributes are padded to a multiple of 4 bytes,
    /// the last 2 bits of this field are always zero.
    pub message_length: u16,
    /// The Magic Cookie field MUST contain the fixed value 0x2112A442 in network byte order.
    magic_cookie: u32,
    pub transaction_id: TransactionId,
}

impl fmt::Debug for MessageHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?}, message_length: {}, transaction_id: {:?}",
            self.stun_message_type, self.message_length, self.transaction_id
        )
    }
}

impl MessageHeader {
    pub fn new(message_type: MessageType, transaction_id: TransactionId) -> Self {
        Self {
            reserved_zero_2_bits: 0,
            stun_message_type: message_type,
            message_length: 0,
            magic_cookie: MAGIC_COOKIE,
            transaction_id,
        }
    }
}

impl FixedPacket for MessageHeader {
    fn bytes_count() -> usize {
        20
    }
}

impl<R: io::Read> ReadFrom<R> for MessageHeader {
    type Error = StunMessageError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let message_type = reader.read_u16::<BigEndian>()?;
        if (message_type >> 14) != 0 {
            return Err(StunMessageError::SyntaxError(format!(
                "the first two bits is not 0: {}",
                message_type
            )));
        }
        let stun_message_type = MessageType::try_from(message_type)?;
        let message_length = reader.read_u16::<BigEndian>()?;
        if !message_length.is_multiple_of(4) {
            return Err(StunMessageError::SyntaxError(format!(
                "message length is not multiple of 4: {}",
                message_length
            )));
        }
        let magic_cookie = reader.read_u32::<BigEndian>()?;
        if magic_cookie != MAGIC_COOKIE {
            return Err(StunMessageError::SyntaxError(format!(
                "wrong magic cookie: {}",
                magic_cookie
            )));
        }

        let transaction_id = TransactionId::read_from(reader)?;
        Ok(Self {
            reserved_zero_2_bits: 0,
            stun_message_type,
            message_length,
            magic_cookie,
            transaction_id,
        })
    }
}

impl<W: io::Write> WriteTo<W> for MessageHeader {
    type Error = StunMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u16::<BigEndian>((&self.stun_message_type).into())?;
        writer.write_u16::<BigEndian>(self.message_length)?;
        writer.write_u32::<BigEndian>(self.magic_cookie)?;
        self.transaction_id.write_to(writer)?;
        Ok(())
    }
}
