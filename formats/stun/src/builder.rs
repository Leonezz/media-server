use crate::{
    attributes::{
        AttributeExtDynamic, AttributeExtStatic, AttributeFactory, RawAttribute,
        rfc8489::{
            self, FingerPrintAttribute, MessageIntegrityAttribute, MessageIntegritySHA256Attribute,
        },
    },
    errors::StunMessageResult,
    header::{MessageClass, MessageHeader, MessageType, TransactionId},
    message::Message,
    methods::{self, CloneableMethodExt},
};

#[derive(Debug, Default)]
pub struct MessageBuilder {
    transaction_id: Option<TransactionId>,
    message_method: Option<Box<dyn CloneableMethodExt>>,
    message_class: Option<MessageClass>,
    attributes: Vec<RawAttribute>,
}

impl MessageBuilder {
    pub fn transaction_id(mut self, transaction_id: TransactionId) -> Self {
        self.transaction_id = Some(transaction_id);
        self
    }

    pub fn binding(mut self) -> Self {
        self.message_method = Some(Box::new(methods::rfc8489::BINDING));
        self
    }

    pub fn method_ext<M: CloneableMethodExt + 'static>(mut self, method: M) -> Self {
        self.message_method = Some(Box::new(method));
        self
    }

    pub fn method(mut self, method: Box<dyn CloneableMethodExt>) -> Self {
        self.message_method = Some(method);
        self
    }

    pub fn request(mut self) -> Self {
        self.message_class = Some(MessageClass::Request);
        self
    }

    pub fn success(mut self) -> Self {
        self.message_class = Some(MessageClass::SuccessResponse);
        self
    }

    pub fn error(mut self) -> Self {
        self.message_class = Some(MessageClass::ErrorResponse);
        self
    }

    pub fn error_mut(&mut self) -> &mut Self {
        self.message_class = Some(MessageClass::ErrorResponse);
        self
    }

    pub fn indication(mut self) -> Self {
        self.message_class = Some(MessageClass::Indication);
        self
    }

    pub fn attribute<A: AttributeFactory>(mut self, attr: A) -> StunMessageResult<Self> {
        self.attribute_mut(attr)?;
        Ok(self)
    }

    pub fn attribute_mut<A: AttributeFactory>(&mut self, attr: A) -> StunMessageResult<&mut Self> {
        if self.transaction_id.is_none() {
            return Err(crate::errors::StunMessageError::BuilderError(format!(
                "unable to set attribute before transaction id is set"
            )));
        }
        match A::STATIC_ATTR_TYPE {
            MessageIntegrityAttribute::STATIC_ATTR_TYPE => {
                let attr = Box::new(attr)
                    .into_any()
                    .downcast::<MessageIntegrityAttribute>()
                    .expect(&format!(
                        "an attribute with attr_type {} must be {}",
                        MessageIntegrityAttribute::STATIC_ATTR_TYPE,
                        MessageIntegrityAttribute::STATIC_NAME,
                    ));
                return self.message_integrity_mut(attr);
            }
            MessageIntegritySHA256Attribute::STATIC_ATTR_TYPE => {
                let attr = Box::new(attr)
                    .into_any()
                    .downcast::<MessageIntegritySHA256Attribute>()
                    .expect(&format!(
                        "an attribute with attr_type {} must be {}",
                        MessageIntegritySHA256Attribute::STATIC_ATTR_TYPE,
                        MessageIntegritySHA256Attribute::STATIC_NAME,
                    ));
                return self.message_integrity_sha256_mut(attr);
            }
            FingerPrintAttribute::STATIC_ATTR_TYPE => return self.finger_print_mut(),
            _ => {
                self.attributes
                    .push(attr.into_raw_attr(&self.transaction_id.unwrap()));
                return Ok(self);
            }
        }
    }

    pub fn message_integrity(
        mut self,
        attr: Box<rfc8489::MessageIntegrityAttribute>,
    ) -> StunMessageResult<Self> {
        self.message_integrity_mut(attr)?;
        Ok(self)
    }

    pub(crate) fn message_integrity_mut(
        &mut self,
        attr: Box<rfc8489::MessageIntegrityAttribute>,
    ) -> StunMessageResult<&mut Self> {
        if self.transaction_id.is_none() {
            return Err(crate::errors::StunMessageError::BuilderError(format!(
                "unable to set {:?} before transaction id is set",
                attr
            )));
        }
        if self.message_method.is_none() {
            return Err(crate::errors::StunMessageError::BuilderError(format!(
                "unable to set {:?} before message method is set",
                attr
            )));
        }
        if self.message_class.is_none() {
            return Err(crate::errors::StunMessageError::BuilderError(format!(
                "unable to set {:?} before message class is set",
                attr
            )));
        }
        if self
            .attributes
            .iter()
            .any(|item| matches!(item.attr_type, MessageIntegrityAttribute::STATIC_ATTR_TYPE))
        {
            return Err(crate::errors::StunMessageError::BuilderError(format!(
                "multiple {:?} attr is not allowed: {:?}",
                attr.get_type(),
                attr
            )));
        }
        if self
            .attributes
            .iter()
            .any(|item| matches!(item.attr_type, FingerPrintAttribute::STATIC_ATTR_TYPE))
        {
            return Err(crate::errors::StunMessageError::InvalidMessage(format!(
                "{:?} after finger print is not allowed",
                attr
            )));
        }
        let dummy_message = Message::new(
            MessageHeader::new(
                MessageType::new_method(
                    self.message_method.clone().unwrap(),
                    self.message_class.unwrap(),
                ),
                self.transaction_id.unwrap(),
            ),
            self.attributes.clone(),
        );
        self.attributes.push(
            attr.sign(dummy_message)
                .into_raw_attr(&self.transaction_id.unwrap()),
        );
        Ok(self)
    }

    pub fn message_integrity_sha256(
        mut self,
        attr: Box<rfc8489::MessageIntegritySHA256Attribute>,
    ) -> StunMessageResult<Self> {
        self.message_integrity_sha256_mut(attr)?;
        Ok(self)
    }

    pub(crate) fn message_integrity_sha256_mut(
        &mut self,
        attr: Box<rfc8489::MessageIntegritySHA256Attribute>,
    ) -> StunMessageResult<&mut Self> {
        if self.transaction_id.is_none() {
            return Err(crate::errors::StunMessageError::InvalidMessage(format!(
                "unable to set {:?} before transaction id is set",
                attr
            )));
        }
        if self.message_method.is_none() {
            return Err(crate::errors::StunMessageError::InvalidMessage(format!(
                "unable to set {:?} before message method is set",
                attr
            )));
        }
        if self.message_class.is_none() {
            return Err(crate::errors::StunMessageError::InvalidMessage(format!(
                "unable to set {:?} before message class is set",
                attr
            )));
        }
        if self.attributes.iter().any(|item| {
            matches!(
                item.attr_type,
                MessageIntegritySHA256Attribute::STATIC_ATTR_TYPE
            )
        }) {
            return Err(crate::errors::StunMessageError::InvalidMessage(format!(
                "multiple {:?} attr is not allowed: {:?}",
                attr.get_type(),
                attr
            )));
        }

        if self
            .attributes
            .iter()
            .any(|item| matches!(item.attr_type, FingerPrintAttribute::STATIC_ATTR_TYPE))
        {
            return Err(crate::errors::StunMessageError::InvalidMessage(format!(
                "{:?} after finger print is not allowed",
                attr
            )));
        }

        let dummy_message = Message::new(
            MessageHeader::new(
                MessageType::new_method(
                    self.message_method.clone().unwrap(),
                    self.message_class.unwrap(),
                ),
                self.transaction_id.unwrap(),
            ),
            self.attributes.clone(),
        );
        self.attributes.push(
            attr.sign(dummy_message)
                .into_raw_attr(&self.transaction_id.unwrap()),
        );
        Ok(self)
    }

    pub fn finger_print(mut self) -> StunMessageResult<Self> {
        self.finger_print_mut()?;
        Ok(self)
    }

    pub(crate) fn finger_print_mut(&mut self) -> StunMessageResult<&mut Self> {
        let attr = FingerPrintAttribute::new_dummy();
        if self.transaction_id.is_none() {
            return Err(crate::errors::StunMessageError::InvalidMessage(format!(
                "unable to set {:?} before transaction id is set",
                attr
            )));
        }
        if self.message_method.is_none() {
            return Err(crate::errors::StunMessageError::InvalidMessage(format!(
                "unable to set {:?} before message method is set",
                attr
            )));
        }
        if self.message_class.is_none() {
            return Err(crate::errors::StunMessageError::InvalidMessage(format!(
                "unable to set {:?} before message class is set",
                attr
            )));
        }
        if self
            .attributes
            .iter()
            .any(|item| matches!(item.attr_type, FingerPrintAttribute::STATIC_ATTR_TYPE))
        {
            return Err(crate::errors::StunMessageError::InvalidMessage(format!(
                "multiple {:?} is not allowed: {:?}",
                attr.get_type(),
                attr
            )));
        }

        let message = Message::new(
            MessageHeader::new(
                MessageType::new_method(
                    self.message_method.clone().unwrap(),
                    self.message_class.unwrap(),
                ),
                self.transaction_id.unwrap(),
            ),
            self.attributes.clone(),
        );
        let real_attr = rfc8489::FingerPrintAttribute::sign(message);
        self.attributes
            .push(real_attr.into_raw_attr(&self.transaction_id.unwrap()));
        Ok(self)
    }

    pub fn build(self) -> StunMessageResult<Message> {
        if self.transaction_id.is_none() {
            return Err(crate::errors::StunMessageError::InvalidMessage(
                "unable to build message before transaction id is set".to_owned(),
            ));
        }
        if self.message_method.is_none() {
            return Err(crate::errors::StunMessageError::InvalidMessage(
                "unable to build before message method is set".to_owned(),
            ));
        }
        if self.message_class.is_none() {
            return Err(crate::errors::StunMessageError::InvalidMessage(
                "unable to build before message class is set".to_owned(),
            ));
        }
        let message = Message::new(
            MessageHeader::new(
                MessageType::new_method(
                    self.message_method.clone().unwrap(),
                    self.message_class.unwrap(),
                ),
                self.transaction_id.unwrap(),
            ),
            self.attributes,
        );

        message.check()?;
        Ok(message)
    }
}
