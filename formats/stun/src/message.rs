use std::io::{self, BufRead};

use num::ToPrimitive;
use utils::traits::{
    dynamic_sized_packet::DynamicSizedPacket, fixed_packet::FixedPacket, reader::ReadFrom,
    writer::WriteTo,
};

use crate::{
    attribute::{AttrType, STUNAttribute, STUNAttributeExt, STUNRawAttribute},
    builder::STUNMessageBuilder,
    errors::{STUNMessageError, STUNMessageResult},
    header::STUNMessageHeader,
};

#[derive(Debug, Clone)]
pub struct STUNMessage {
    header: STUNMessageHeader,
    attributes: Vec<STUNAttribute>,
}

impl DynamicSizedPacket for STUNMessage {
    fn get_packet_bytes_count(&self) -> usize {
        STUNMessageHeader::bytes_count()
            + self
                .attributes
                .iter()
                .fold(0, |prev, item| prev + item.get_packet_bytes_count())
    }
}

impl STUNMessage {
    pub fn builder() -> STUNMessageBuilder {
        Default::default()
    }

    pub(crate) fn new(header: STUNMessageHeader, attrs: Vec<STUNAttribute>) -> Self {
        STUNMessage {
            header,
            attributes: attrs,
        }
    }

    pub fn attributes(&self) -> &Vec<STUNAttribute> {
        &self.attributes
    }

    pub fn header(&self) -> &STUNMessageHeader {
        &self.header
    }

    pub fn message_class(&self) -> crate::header::STUNMessageClass {
        self.header.stun_message_type.message_class
    }

    pub fn message_method(&self) -> crate::methods::STUNMethod {
        self.header.stun_message_type.method
    }

    pub fn prepare_dummy_message_bytes(mut self, attr: STUNAttribute) -> Self {
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

    pub fn get_attribute(&self, attr_type: AttrType) -> Option<&STUNAttribute> {
        self.attributes
            .iter()
            .find(|item| item.get_type() == attr_type)
    }

    pub fn require(&self, attr_type: AttrType) -> STUNMessageResult<()> {
        if self.get_attribute(attr_type).is_none() {
            return Err(STUNMessageError::InvalidMessage(format!(
                "no {:?} found in message: {:?}",
                attr_type, self
            )));
        }
        Ok(())
    }
}

impl<W: io::Write> WriteTo<W> for STUNMessage {
    type Error = STUNMessageError;
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

impl<R: io::Read> ReadFrom<R> for STUNMessage {
    type Error = STUNMessageError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let header = STUNMessageHeader::read_from(reader)?;
        let mut remaining_bytes = vec![0_u8; header.message_length as usize];
        reader.read_exact(&mut remaining_bytes)?;
        let mut bytes = remaining_bytes.as_slice();
        let mut attributes = Vec::new();
        while bytes.has_data_left()? {
            let raw_attr = STUNRawAttribute::read_from(&mut bytes)?;
            attributes.push(STUNAttribute::from_raw_attr(
                raw_attr,
                &header.transaction_id,
            )?);
        }
        let message = Self { header, attributes };
        message.message_method().check(&message)?;
        Ok(message)
    }
}
