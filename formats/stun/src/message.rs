use std::{
    fmt,
    io::{self, BufRead, Read},
};

use num::ToPrimitive;
use tokio_util::{
    bytes::{Buf, BufMut},
    codec::{Decoder, Encoder},
};
use utils::traits::{
    self, dynamic_sized_packet::DynamicSizedPacket, fixed_packet::FixedPacket, reader::ReadFrom,
    writer::WriteTo,
};

use crate::{
    attribute::{AttrType, Attribute, AttributeExt, RawAttribute},
    builder::STUNMessageBuilder,
    errors::{STUNMessageResult, StunMessageError},
    header::{STUNMessageHeader, TransactionId},
};

#[derive(Clone)]
pub struct Message {
    header: STUNMessageHeader,
    attributes: Vec<Attribute>,
}

impl traits::protocol_message::ProtocolMessage for Message {
    type Codec = MessageFramed;
    type Error = StunMessageError;
    type In = Message;
    type Out = Message;
    fn codec() -> Self::Codec {
        MessageFramed {}
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
        STUNMessageHeader::bytes_count()
            + self
                .attributes
                .iter()
                .fold(0, |prev, item| prev + item.get_packet_bytes_count())
    }
}

impl Message {
    pub fn builder() -> STUNMessageBuilder {
        Default::default()
    }

    pub(crate) fn new(header: STUNMessageHeader, attrs: Vec<Attribute>) -> Self {
        Message {
            header,
            attributes: attrs,
        }
    }

    pub fn attributes(&self) -> &Vec<Attribute> {
        &self.attributes
    }

    pub fn header(&self) -> &STUNMessageHeader {
        &self.header
    }

    pub fn transaction_id(&self) -> &TransactionId {
        &self.header.transaction_id
    }
    pub fn message_class(&self) -> crate::header::MessageClass {
        self.header.stun_message_type.message_class
    }

    pub fn message_method(&self) -> crate::methods::Method {
        self.header.stun_message_type.method
    }

    pub fn prepare_dummy_message_bytes(mut self, attr: Attribute) -> Self {
        self.attributes.push(attr);
        self
    }

    pub fn has_authentication(&self) -> bool {
        self.attributes.iter().any(|item| {
            let attr_type = item.get_type();
            matches!(attr_type, AttrType::MessageIntegrity)
                || matches!(attr_type, AttrType::MessageIntegritySHA256)
        })
    }

    pub fn get_attribute(&self, attr_type: AttrType) -> Option<&Attribute> {
        self.attributes
            .iter()
            .find(|item| item.get_type() == attr_type)
    }

    pub fn get_attribute_ext<Attr: AttributeExt>(&self) -> Option<Attr> {
        self.attributes.iter().find_map(|item| {
            if let Some(static_attr_type) = Attr::STATIC_ATTR_TYPE
                && static_attr_type == item.get_type()
            {
                let raw_attribute = item.clone().into_raw_attr(self.transaction_id());
                return Attr::from_raw_attr(raw_attribute, self.transaction_id()).ok();
            }
            None
        })
    }

    pub fn require(&self, attr_type: AttrType) -> STUNMessageResult<()> {
        if self.get_attribute(attr_type).is_none() {
            return Err(StunMessageError::InvalidMessage(format!(
                "no {:?} found in message: {:?}",
                attr_type, self
            )));
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
        let mut header = self.header;
        header.message_length = body_len.to_u16().unwrap();
        header.write_to(writer)?;
        self.attributes.iter().try_for_each(|item| {
            let raw_attr = item.clone().into_raw_attr(&self.header.transaction_id);
            raw_attr.write_to(writer)?;
            Ok::<(), Self::Error>(())
        })?;
        Ok(())
    }
}

impl<R: io::Read> ReadFrom<R> for Message {
    type Error = StunMessageError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let header = STUNMessageHeader::read_from(reader)?;
        let mut remaining_bytes = vec![0_u8; header.message_length as usize];
        reader.read_exact(&mut remaining_bytes)?;
        let mut bytes = remaining_bytes.as_slice();
        let mut attributes = Vec::new();
        while bytes.has_data_left()? {
            let raw_attr = RawAttribute::read_from(&mut bytes)?;
            attributes.push(Attribute::from_raw_attr(raw_attr, &header.transaction_id)?);
        }
        let message = Self { header, attributes };
        message.message_method().check(&message)?;
        Ok(message)
    }
}

#[derive(Debug)]
pub struct MessageFramed;
impl Encoder<Message> for MessageFramed {
    type Error = StunMessageError;
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
    type Error = StunMessageError;
    type Item = Message;
    fn decode(
        &mut self,
        src: &mut tokio_util::bytes::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < STUNMessageHeader::bytes_count() {
            return Ok(None);
        }
        let (res, position) = {
            let mut cursor = io::Cursor::new(&src);
            let res = Message::read_from(cursor.by_ref());
            (res, cursor.position())
        };
        if let Ok(res) = res {
            src.advance(position as usize);
            return Ok(Some(res));
        }
        Err(res.unwrap_err())
    }
}
