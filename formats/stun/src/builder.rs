use crate::{
    attribute::{STUNAttribute, STUNAttributeExt},
    errors::STUNMessageResult,
    header::{STUNMessageClass, STUNMessageHeader, STUNMessageType, TransactionId},
    message::STUNMessage,
    methods::{STUNMethod, rfc8489::STUNMethodBinding},
    rfc8489::{self, FingerPrintAttribute},
};

#[derive(Debug, Default)]
pub struct STUNMessageBuilder {
    transaction_id: Option<TransactionId>,
    message_method: Option<STUNMethod>,
    message_class: Option<STUNMessageClass>,
    attributes: Vec<STUNAttribute>,
}

impl STUNMessageBuilder {
    pub fn transaction_id(mut self, transaction_id: TransactionId) -> Self {
        self.transaction_id = Some(transaction_id);
        self
    }

    pub fn binding(mut self) -> Self {
        self.message_method = Some(STUNMethod::Binding(STUNMethodBinding {}));
        self
    }

    pub fn request(mut self) -> Self {
        self.message_class = Some(STUNMessageClass::Request);
        self
    }

    pub fn success(mut self) -> Self {
        self.message_class = Some(STUNMessageClass::SuccessResponse);
        self
    }

    pub fn error(mut self) -> Self {
        self.message_class = Some(STUNMessageClass::ErrorResponse);
        self
    }

    pub fn indication(mut self) -> Self {
        self.message_class = Some(STUNMessageClass::Indication);
        self
    }

    pub fn attribute(mut self, attr: STUNAttribute) -> STUNMessageResult<Self> {
        match attr {
            STUNAttribute::MessageIntegrity(integrity) => self.message_integrity(integrity),
            STUNAttribute::MessageIntegritySHA256(integrity) => {
                self.message_integrity_sha256(integrity)
            }
            STUNAttribute::FingerPrint(_) => self.finger_print(),
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
            return Err(crate::errors::STUNMessageError::InvalidMessage(format!(
                "unable to set {:?} before transaction id is set",
                attr
            )));
        }
        if self.message_method.is_none() {
            return Err(crate::errors::STUNMessageError::InvalidMessage(format!(
                "unable to set {:?} before message method is set",
                attr
            )));
        }
        if self.message_class.is_none() {
            return Err(crate::errors::STUNMessageError::InvalidMessage(format!(
                "unable to set {:?} before message class is set",
                attr
            )));
        }
        if self
            .attributes
            .iter()
            .any(|item| matches!(&item, STUNAttribute::MessageIntegrity(_)))
        {
            return Err(crate::errors::STUNMessageError::InvalidMessage(format!(
                "multiple {:?} attr is not allowed: {:?}",
                attr.get_type(),
                attr
            )));
        }
        if self
            .attributes
            .iter()
            .any(|item| matches!(&item, STUNAttribute::FingerPrint(_)))
        {
            return Err(crate::errors::STUNMessageError::InvalidMessage(format!(
                "{:?} after finger print is not allowed",
                attr
            )));
        }
        let dummy_message = STUNMessage::new(
            STUNMessageHeader::new(
                STUNMessageType::new(self.message_method.unwrap(), self.message_class.unwrap()),
                self.transaction_id.unwrap(),
            ),
            self.attributes.clone(),
        );
        self.attributes
            .push(STUNAttribute::MessageIntegrity(attr.sign(dummy_message)));
        Ok(self)
    }

    pub fn message_integrity_sha256(
        mut self,
        attr: rfc8489::MessageIntegritySHA256Attribute,
    ) -> STUNMessageResult<Self> {
        if self.transaction_id.is_none() {
            return Err(crate::errors::STUNMessageError::InvalidMessage(format!(
                "unable to set {:?} before transaction id is set",
                attr
            )));
        }
        if self.message_method.is_none() {
            return Err(crate::errors::STUNMessageError::InvalidMessage(format!(
                "unable to set {:?} before message method is set",
                attr
            )));
        }
        if self.message_class.is_none() {
            return Err(crate::errors::STUNMessageError::InvalidMessage(format!(
                "unable to set {:?} before message class is set",
                attr
            )));
        }
        if self
            .attributes
            .iter()
            .any(|item| matches!(&item, STUNAttribute::MessageIntegritySHA256(_)))
        {
            return Err(crate::errors::STUNMessageError::InvalidMessage(format!(
                "multiple {:?} attr is not allowed: {:?}",
                attr.get_type(),
                attr
            )));
        }

        if self
            .attributes
            .iter()
            .any(|item| matches!(&item, STUNAttribute::FingerPrint(_)))
        {
            return Err(crate::errors::STUNMessageError::InvalidMessage(format!(
                "{:?} after finger print is not allowed",
                attr
            )));
        }

        let dummy_message = STUNMessage::new(
            STUNMessageHeader::new(
                STUNMessageType::new(self.message_method.unwrap(), self.message_class.unwrap()),
                self.transaction_id.unwrap(),
            ),
            self.attributes.clone(),
        );
        self.attributes.push(STUNAttribute::MessageIntegritySHA256(
            attr.sign(dummy_message),
        ));
        Ok(self)
    }

    pub fn finger_print(mut self) -> STUNMessageResult<Self> {
        let attr = FingerPrintAttribute::new_dummy();
        if self.transaction_id.is_none() {
            return Err(crate::errors::STUNMessageError::InvalidMessage(format!(
                "unable to set {:?} before transaction id is set",
                attr
            )));
        }
        if self.message_method.is_none() {
            return Err(crate::errors::STUNMessageError::InvalidMessage(format!(
                "unable to set {:?} before message method is set",
                attr
            )));
        }
        if self.message_class.is_none() {
            return Err(crate::errors::STUNMessageError::InvalidMessage(format!(
                "unable to set {:?} before message class is set",
                attr
            )));
        }
        if self
            .attributes
            .iter()
            .any(|item| matches!(item, STUNAttribute::FingerPrint(_)))
        {
            return Err(crate::errors::STUNMessageError::InvalidMessage(format!(
                "multiple {:?} is not allowed: {:?}",
                attr.get_type(),
                attr
            )));
        }

        let message = STUNMessage::new(
            STUNMessageHeader::new(
                STUNMessageType::new(self.message_method.unwrap(), self.message_class.unwrap()),
                self.transaction_id.unwrap(),
            ),
            self.attributes.clone(),
        );
        let real_attr = rfc8489::FingerPrintAttribute::sign(message);
        self.attributes.push(STUNAttribute::FingerPrint(real_attr));
        Ok(self)
    }

    pub fn build(self) -> STUNMessageResult<STUNMessage> {
        if self.transaction_id.is_none() {
            return Err(crate::errors::STUNMessageError::InvalidMessage(
                "unable to build message before transaction id is set".to_owned(),
            ));
        }
        if self.message_method.is_none() {
            return Err(crate::errors::STUNMessageError::InvalidMessage(
                "unable to build before message method is set".to_owned(),
            ));
        }
        if self.message_class.is_none() {
            return Err(crate::errors::STUNMessageError::InvalidMessage(
                "unable to build before message class is set".to_owned(),
            ));
        }
        let message = STUNMessage::new(
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
