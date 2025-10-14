use crate::{
    MessageChecker,
    errors::{StunMessageResult, StunMessageError},
    header::TransactionId,
};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use num::ToPrimitive;
use std::{
    any::Any,
    fmt::{self, Debug},
    io,
};
use utils::traits::{
    dynamic_sized_packet::DynamicSizedPacket,
    reader::{ReadFrom, ReadRemainingFrom},
    writer::WriteTo,
};
pub mod rfc8489;
pub fn check_attr_match(from_attr: u16, to_attr: u16) -> StunMessageResult<()> {
    if from_attr != to_attr {
        return Err(StunMessageError::SyntaxError(format!(
            "attr {:?} and {:?} not match",
            from_attr, to_attr
        )));
    }
    Ok(())
}

pub const STUN_ATTRIBUTE_PADDING_SIZE: usize = 4;

pub fn need_padding(len: usize, padding_to: usize) -> bool {
    !len.is_multiple_of(padding_to)
}

pub fn get_after_padding_size(len: usize, padding_to: usize) -> usize {
    assert_ne!(padding_to, 0);
    let size = padding_to * (len / padding_to);
    if size < len {
        return size + padding_to;
    }
    size
}

pub fn make_padding(buf: &mut Vec<u8>, padding_to: usize) {
    let padding_size = get_after_padding_size(buf.len(), padding_to) - buf.len();
    buf.extend(vec![0; padding_size]);
}

///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |             Type              |            Length             |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                        Value (variable)                    ....
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Clone)]
pub struct RawAttribute {
    pub attr_type: u16,
    length: u16,
    pub value: Vec<u8>,
}

impl DynamicSizedPacket for RawAttribute {
    fn get_packet_bytes_count(&self) -> usize {
        2 + // Type
        2 + // Length
        self.length as usize
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
    pub fn new(attr_type: u16, value: Vec<u8>) -> Self {
        assert_eq!(value.len().to_u16(), Some(value.len() as u16));
        Self {
            attr_type,
            length: get_after_padding_size(value.len(), STUN_ATTRIBUTE_PADDING_SIZE) as u16,
            value,
        }
    }

    pub fn into_extension(
        self,
        transaction_id: TransactionId,
    ) -> Option<StunMessageResult<Box<dyn AttributeExtDynamic>>> {
        for entry in inventory::iter::<AttributeExtEntry> {
            if entry.attr_type == self.attr_type {
                return Some((entry.factory)(self, &transaction_id));
            }
        }
        None
    }

    pub fn into_concrete<T>(self, transaction_id: TransactionId) -> Option<StunMessageResult<T>>
    where
        T: AttributeFactory,
    {
        if T::STATIC_ATTR_TYPE == self.attr_type {
            return Some(T::from_raw_attr(self, &transaction_id));
        }
        None
    }
}

impl MessageChecker for RawAttribute {}

impl AttributeExtDynamic for RawAttribute {
    fn get_name(&self) -> &str {
        "unknown"
    }

    fn get_type(&self) -> u16 {
        self.attr_type
    }
}

pub trait AttributeExtStatic: Any + Sized {
    const STATIC_ATTR_TYPE: u16;
    const STATIC_NAME: &'static str;
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

pub trait AttributeFactory: AttributeExtStatic + Sized {
    fn from_raw_attr(
        raw_attr: RawAttribute,
        transaction_id: &TransactionId,
    ) -> Result<Self, StunMessageError>;
    fn into_raw_attr(self, transaction_id: &TransactionId) -> RawAttribute;
}

pub trait AttributeExtDynamic: MessageChecker + Debug + Send + Sync {
    fn get_type(&self) -> u16;
    fn get_name(&self) -> &str;
    fn is_comprehension_required(&self) -> bool {
        u16::from(self.get_type()) < 0x8000
    }
}

pub struct AttributeExtEntry {
    pub attr_type: u16,
    pub factory:
        fn(RawAttribute, &TransactionId) -> StunMessageResult<Box<dyn AttributeExtDynamic>>,
}

inventory::collect!(AttributeExtEntry);

#[macro_export]
macro_rules! define_attribute {
    ($attr_type: tt, $attr: ident, $name: expr) => {
        impl AttributeExtStatic for $attr {
            const STATIC_ATTR_TYPE: u16 = $attr_type;
            const STATIC_NAME: &'static str = $name;
        }

        impl AttributeExtDynamic for $attr {
            fn get_type(&self) -> u16 {
                $attr_type
            }

            fn get_name(&self) -> &str {
                $name
            }
        }

        inventory::submit! {
            $crate::attributes::AttributeExtEntry {
                attr_type: $attr_type,
                factory: |raw_attr, transaction_id| Ok(Box::new($attr::from_raw_attr(raw_attr, transaction_id)?))
            }
        }
    };
}

impl<R: io::Read> ReadFrom<R> for RawAttribute {
    type Error = StunMessageError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let attr_type = reader.read_u16::<BigEndian>()?.into();
        Self::read_remaining_from(attr_type, reader)
    }
}

impl<R: io::Read> ReadRemainingFrom<u16, R> for RawAttribute {
    type Error = StunMessageError;
    fn read_remaining_from(header: u16, reader: &mut R) -> Result<Self, Self::Error> {
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
        debug_assert!(
            self.length
                .is_multiple_of(STUN_ATTRIBUTE_PADDING_SIZE as u16)
        );
        writer.write_u16::<BigEndian>(self.attr_type.into())?;
        writer.write_u16::<BigEndian>(self.value.len().to_u16().unwrap())?;
        writer.write_all(&self.value)?;
        let padding_bytes_len = self.length as usize - self.value.len();
        for _ in 0..padding_bytes_len {
            writer.write_u8(0)?;
        }
        Ok(())
    }
}
