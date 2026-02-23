use std::{
    fmt,
    io::{self, BufRead, Read},
};

use num::ToPrimitive;
use rootcause::{Report, bail};
use tokio_util::{
    bytes::{Buf, BufMut},
    codec::{Decoder, Encoder},
};
use utils::{
    errors::context::{ContextExt, LocationExt},
    traits::{
        self,
        dynamic_sized_packet::DynamicSizedPacket,
        fixed_packet::FixedPacket,
        reader::{ReadFrom, ReadRemainingFrom, TryReadRemainingFrom},
        writer::WriteTo,
    },
};

use crate::{
    attributes::{
        AttributeExtDynamic, AttributeExtStatic, AttributeFactory, RawAttribute,
        rfc8489::{MessageIntegrityAttribute, MessageIntegritySHA256Attribute},
    },
    builder::MessageBuilder,
    errors::{StunMessageError, StunMessageResult},
    header::{MessageHeader, TransactionId},
    methods::CloneableMethodExt,
};

#[derive(Clone)]
pub struct Message {
    header: MessageHeader,
    attributes: Vec<RawAttribute>,
}

impl traits::protocol_message::ProtocolMessage for Message {
    type Codec = MessageFramed;
    type Error = Report;
    type In = Message;
    type Out = Message;
    fn codec() -> Self::Codec {
        MessageFramed {}
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, ", self.header)?;
        self.attributes
            .iter()
            .try_for_each(|item| write!(f, " attr: {} ", item))
    }
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{:?}", self.header)?;
        self.attributes
            .iter()
            .try_for_each(|item| writeln!(f, "{:?}", item))?;
        Ok(())
    }
}

impl DynamicSizedPacket for Message {
    fn get_packet_bytes_count(&self) -> usize {
        MessageHeader::bytes_count()
            + self
                .attributes
                .iter()
                .fold(0, |prev, item| prev + item.get_packet_bytes_count())
    }
}

impl Message {
    pub fn builder() -> MessageBuilder {
        Default::default()
    }

    pub(crate) fn new(header: MessageHeader, attrs: Vec<RawAttribute>) -> Self {
        Message {
            header,
            attributes: attrs,
        }
    }

    pub fn attributes(&self) -> &Vec<RawAttribute> {
        &self.attributes
    }

    pub fn attributes_ext<Attr: AttributeExtDynamic + AttributeFactory>(
        &self,
    ) -> impl Iterator<Item = Attr> {
        self.attributes
            .iter()
            .filter(|item| item.attr_type == Attr::STATIC_ATTR_TYPE)
            .map(|item| Attr::from_raw_attr(item.clone(), self.transaction_id()).ok())
            .filter(|item| item.is_some())
            .map(|item| item.unwrap())
    }

    pub fn header(&self) -> &MessageHeader {
        &self.header
    }

    pub fn transaction_id(&self) -> &TransactionId {
        &self.header.transaction_id
    }
    pub fn message_class(&self) -> crate::header::MessageClass {
        self.header.stun_message_type.message_class
    }

    pub fn message_method(&self) -> &Box<dyn CloneableMethodExt> {
        &self.header.stun_message_type.method
    }

    pub fn prepare_dummy_message_bytes<A: AttributeFactory>(mut self, attr: A) -> Self {
        self.attributes
            .push(attr.into_raw_attr(self.transaction_id()));
        self
    }

    pub fn has_authentication(&self) -> bool {
        self.attributes.iter().any(|item| {
            let attr_type = item.attr_type;
            matches!(attr_type, MessageIntegrityAttribute::STATIC_ATTR_TYPE)
                || matches!(attr_type, MessageIntegritySHA256Attribute::STATIC_ATTR_TYPE)
        })
    }

    pub fn get_attribute(&self, attr_type: u16) -> Option<&RawAttribute> {
        self.attributes
            .iter()
            .find(|item| item.attr_type == attr_type)
    }

    pub fn get_attribute_ext<Attr: AttributeExtDynamic + AttributeFactory>(&self) -> Option<Attr> {
        self.attributes.iter().find_map(|item| {
            if Attr::STATIC_ATTR_TYPE == item.attr_type {
                let raw_attribute = item.clone();
                return Attr::from_raw_attr(raw_attribute, self.transaction_id())
                    .resource("transaction", self.transaction_id())
                    .ok();
            }
            None
        })
    }

    pub fn require(&self, attr_type: u16) -> StunMessageResult<()> {
        if self.get_attribute(attr_type).is_none() {
            bail!(StunMessageError::InvalidMessage(format!(
                "no {:?} found in message: {:?}",
                attr_type, self
            )));
        }
        Ok(())
    }

    pub fn require_ext<A: AttributeExtStatic>(&self) -> StunMessageResult<()> {
        self.require(A::STATIC_ATTR_TYPE).trace()
    }

    pub fn count(&self, attr_type: u16) -> usize {
        self.attributes().iter().fold(0, |prev, item| {
            if item.attr_type == attr_type {
                return prev + 1;
            }
            prev
        })
    }

    pub fn check(&self) -> StunMessageResult<()> {
        self.message_class()
            .check(self)
            .operation("validating STUN message")
            .trace()?;
        self.message_method().check(self).trace()?;
        let mut unknown_attributes = vec![];
        self.attributes().iter().try_for_each(|item| {
            let ext = item.clone().into_extension(*self.transaction_id());
            if let Some(attr) = ext {
                attr.trace()?.check(self).trace()?;
            } else if item.is_comprehension_required() {
                unknown_attributes.push(item.attr_type);
            }
            Ok::<(), Report>(())
        })?;
        if !unknown_attributes.is_empty() {
            bail!(StunMessageError::UnknownAttributes(unknown_attributes))
        }
        Ok(())
    }
}

impl<W: io::Write> WriteTo<W> for Message {
    type Error = StunMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let body_len = self
            .attributes
            .iter()
            .fold(0, |prev, item| prev + item.get_packet_bytes_count());
        let mut header = self.header.clone();
        header.message_length = body_len.to_u16().unwrap();
        header.write_to(writer)?;
        self.attributes.iter().try_for_each(|item| {
            item.write_to(writer)?;
            Ok::<(), Self::Error>(())
        })?;
        Ok(())
    }
}

impl<R: io::Read> ReadFrom<R> for Message {
    type Error = Report;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let header = MessageHeader::read_from(reader)?;
        let message = Self::read_remaining_from(header.clone(), reader)
            .trace_with(|| format!("header: {}", header))?;
        Ok(message)
    }
}

// for turn ChannelData interleaving
impl<R: io::Read> ReadRemainingFrom<u8, R> for Message {
    type Error = Report;
    fn read_remaining_from(header: u8, reader: &mut R) -> Result<Self, Self::Error> {
        assert!(header <= 3);
        let mut message_header_bytes = vec![0; MessageHeader::bytes_count()];
        message_header_bytes[0] = header;
        reader
            .read_exact(&mut message_header_bytes[1..])
            .map_err(StunMessageError::from)?;
        let message_header = MessageHeader::read_from(&mut message_header_bytes.reader())?;
        Self::read_remaining_from(message_header, reader)
            .operation("reading STUN message from channel-data interleaving")
            .trace()
    }
}

impl<R: AsRef<[u8]>> TryReadRemainingFrom<u8, R> for Message {
    type Error = Report;
    fn try_read_remaining_from(
        header: u8,
        reader: &mut io::Cursor<R>,
    ) -> Result<Option<Self>, Self::Error> {
        assert!(header <= 3);
        if reader.remaining() < MessageHeader::bytes_count() - 1 {
            return Ok(None);
        }
        let mut message_header_bytes = vec![0; MessageHeader::bytes_count()];
        message_header_bytes[0] = header;
        reader
            .read_exact(&mut message_header_bytes[1..])
            .map_err(StunMessageError::from)?;
        let message_header = MessageHeader::read_from(&mut message_header_bytes.reader())?;
        Self::try_read_remaining_from(message_header, reader).trace()
    }
}

impl<R: AsRef<[u8]>> TryReadRemainingFrom<MessageHeader, R> for Message {
    type Error = Report;
    fn try_read_remaining_from(
        header: MessageHeader,
        reader: &mut io::Cursor<R>,
    ) -> Result<Option<Self>, Self::Error> {
        if reader.remaining() < header.message_length as usize {
            return Ok(None);
        }
        let mut remaining_bytes = vec![0_u8; header.message_length as usize];
        reader
            .read_exact(&mut remaining_bytes)
            .map_err(StunMessageError::from)?;
        let mut bytes = remaining_bytes.as_slice();
        let mut attributes = Vec::new();
        while bytes.has_data_left().map_err(StunMessageError::from)? {
            let raw_attr = RawAttribute::read_from(&mut bytes)?;
            attributes.push(raw_attr);
        }
        let message = Self { header, attributes };
        Ok(Some(message))
    }
}

impl<R: io::Read> ReadRemainingFrom<MessageHeader, R> for Message {
    type Error = Report;
    fn read_remaining_from(header: MessageHeader, reader: &mut R) -> Result<Self, Self::Error> {
        let mut remaining_bytes = vec![0_u8; header.message_length as usize];
        reader
            .read_exact(&mut remaining_bytes)
            .map_err(StunMessageError::from)?;
        let mut bytes = remaining_bytes.as_slice();
        let mut attributes = Vec::new();
        while bytes.has_data_left().map_err(StunMessageError::from)? {
            let raw_attr = RawAttribute::read_from(&mut bytes)?;
            attributes.push(raw_attr);
        }
        let message = Self { header, attributes };
        Ok(message)
    }
}

#[derive(Debug)]
pub struct MessageFramed;
impl Encoder<Message> for MessageFramed {
    type Error = Report;
    fn encode(
        &mut self,
        item: Message,
        dst: &mut tokio_util::bytes::BytesMut,
    ) -> Result<(), Self::Error> {
        item.write_to(&mut dst.writer())?;
        Ok(())
    }
}

impl Decoder for MessageFramed {
    type Error = Report;
    type Item = Message;
    fn decode(
        &mut self,
        src: &mut tokio_util::bytes::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < MessageHeader::bytes_count() {
            return Ok(None);
        }
        let (res, position) = {
            let mut cursor = io::Cursor::new(&src);
            let res = Message::read_from(cursor.by_ref()).trace();
            (res, cursor.position())
        };
        if let Ok(res) = res {
            src.advance(position as usize);
            return Ok(Some(res));
        }
        Err(res.unwrap_err().into())
    }
}
