use crate::{
    attribute::{Attribute, AttributeExt},
    errors::STUNMessageResult,
    header::{MessageClass, STUNMessageHeader, STUNMessageType, TransactionId},
    message::Message,
    methods::{Method, rfc8489::STUNMethodBinding},
    rfc8489::{self, FingerPrintAttribute},
};

#[derive(Debug, Default)]
pub struct STUNMessageBuilder {
    transaction_id: Option<TransactionId>,
    message_method: Option<Method>,
    message_class: Option<MessageClass>,
    attributes: Vec<Attribute>,
}

impl STUNMessageBuilder {
    pub fn transaction_id(mut self, transaction_id: TransactionId) -> Self {
        self.transaction_id = Some(transaction_id);
        self
    }

    pub fn binding(mut self) -> Self {
        self.message_method = Some(Method::Binding(STUNMethodBinding {}));
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

    pub fn indication(mut self) -> Self {
        self.message_class = Some(MessageClass::Indication);
        self
    }

    pub fn attribute(mut self, attr: Attribute) -> STUNMessageResult<Self> {
        match attr {
            Attribute::MessageIntegrity(integrity) => self.message_integrity(integrity),
            Attribute::MessageIntegritySHA256(integrity) => {
                self.message_integrity_sha256(integrity)
            }
            Attribute::FingerPrint(_) => self.finger_print(),
            _ => {
                self.attributes.push(attr);
                Ok(self)
            }
        }
    }

    fn message_integrity(
        mut self,
        attr: rfc8489::MessageIntegrityAttribute,
    ) -> STUNMessageResult<Self> {
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
            .any(|item| matches!(&item, Attribute::MessageIntegrity(_)))
        {
            return Err(crate::errors::StunMessageError::InvalidMessage(format!(
                "multiple {:?} attr is not allowed: {:?}",
                attr.get_type(),
                attr
            )));
        }
        if self
            .attributes
            .iter()
            .any(|item| matches!(&item, Attribute::FingerPrint(_)))
        {
            return Err(crate::errors::StunMessageError::InvalidMessage(format!(
                "{:?} after finger print is not allowed",
                attr
            )));
        }
        let dummy_message = Message::new(
            STUNMessageHeader::new(
                STUNMessageType::new(self.message_method.unwrap(), self.message_class.unwrap()),
                self.transaction_id.unwrap(),
            ),
            self.attributes.clone(),
        );
        self.attributes
            .push(Attribute::MessageIntegrity(attr.sign(dummy_message)));
        Ok(self)
    }

    pub fn message_integrity_sha256(
        mut self,
        attr: rfc8489::MessageIntegritySHA256Attribute,
    ) -> STUNMessageResult<Self> {
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
            .any(|item| matches!(&item, Attribute::MessageIntegritySHA256(_)))
        {
            return Err(crate::errors::StunMessageError::InvalidMessage(format!(
                "multiple {:?} attr is not allowed: {:?}",
                attr.get_type(),
                attr
            )));
        }

        if self
            .attributes
            .iter()
            .any(|item| matches!(&item, Attribute::FingerPrint(_)))
        {
            return Err(crate::errors::StunMessageError::InvalidMessage(format!(
                "{:?} after finger print is not allowed",
                attr
            )));
        }

        let dummy_message = Message::new(
            STUNMessageHeader::new(
                STUNMessageType::new(self.message_method.unwrap(), self.message_class.unwrap()),
                self.transaction_id.unwrap(),
            ),
            self.attributes.clone(),
        );
        self.attributes.push(Attribute::MessageIntegritySHA256(
            attr.sign(dummy_message),
        ));
        Ok(self)
    }

    pub fn finger_print(mut self) -> STUNMessageResult<Self> {
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
            .any(|item| matches!(item, Attribute::FingerPrint(_)))
        {
            return Err(crate::errors::StunMessageError::InvalidMessage(format!(
                "multiple {:?} is not allowed: {:?}",
                attr.get_type(),
                attr
            )));
        }

        let message = Message::new(
            STUNMessageHeader::new(
                STUNMessageType::new(self.message_method.unwrap(), self.message_class.unwrap()),
                self.transaction_id.unwrap(),
            ),
            self.attributes.clone(),
        );
        let real_attr = rfc8489::FingerPrintAttribute::sign(message);
        self.attributes.push(Attribute::FingerPrint(real_attr));
        Ok(self)
    }

    pub fn build(self) -> STUNMessageResult<Message> {
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
            STUNMessageHeader::new(
                STUNMessageType::new(self.message_method.unwrap(), self.message_class.unwrap()),
                self.transaction_id.unwrap(),
            ),
            self.attributes,
        );

        message.message_method().check(&message)?;
        Ok(message)
    }
}
