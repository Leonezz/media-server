use std::fmt;

use utils::traits::dynamic_sized_packet::DynamicSizedPacket;

use crate::{
    attribute::AttributeExt,
    attributes::{STUN_ATTRIBUTE_PADDING_SIZE, check_attr_match, get_after_padding_size},
    errors::STUNMessageResult,
};

#[derive(Clone)]
pub struct AlternateDomainAttribute {
    domain: String,
}

impl AlternateDomainAttribute {
    pub fn new(domain: String) -> STUNMessageResult<Self> {
        if domain.len() > ALTERNATE_DOMAIN_MAX_LEN {
            return Err(crate::errors::StunMessageError::SyntaxError(format!(
                "value length for {:?}: {} exceeds max length: {}",
                Self::STATIC_ATTR_TYPE.unwrap(),
                domain.len(),
                ALTERNATE_DOMAIN_MAX_LEN
            )));
        }

        if !domain.is_ascii() {
            return Err(crate::errors::StunMessageError::SyntaxError(format!(
                "domain name of {:?} attr is not an ascii string: {}",
                Self::STATIC_ATTR_TYPE.unwrap(),
                domain,
            )));
        }

        Ok(Self { domain })
    }

    pub fn domain(&self) -> &String {
        &self.domain
    }
}

impl fmt::Debug for AlternateDomainAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AttrType: {:?}, value: {}", self.get_type(), self.domain)
    }
}

impl DynamicSizedPacket for AlternateDomainAttribute {
    fn get_packet_bytes_count(&self) -> usize {
        get_after_padding_size(self.domain.len(), STUN_ATTRIBUTE_PADDING_SIZE)
    }
}

// It MUST be a valid DNS name [RFC1123] (including A-labels [RFC5890]) of 255 or fewer ASCII characters.
pub const ALTERNATE_DOMAIN_MAX_LEN: usize = 255;

impl AttributeExt for AlternateDomainAttribute {
    const STATIC_ATTR_TYPE: Option<crate::attribute::AttrType> =
        Some(crate::attribute::AttrType::AlternateDomain);
    fn get_type(&self) -> crate::attribute::AttrType {
        Self::STATIC_ATTR_TYPE.unwrap()
    }

    fn from_raw_attr(
        raw_attr: crate::attribute::RawAttribute,
        _transaction_id: &crate::header::TransactionId,
    ) -> Result<Self, crate::errors::StunMessageError> {
        check_attr_match(raw_attr.attr_type, Self::STATIC_ATTR_TYPE.unwrap())?;
        if raw_attr.value.len() > ALTERNATE_DOMAIN_MAX_LEN {
            return Err(crate::errors::StunMessageError::SyntaxError(format!(
                "value length for {:?}: {} exceeds max length: {}",
                Self::STATIC_ATTR_TYPE.unwrap(),
                raw_attr.value.len(),
                ALTERNATE_DOMAIN_MAX_LEN
            )));
        }
        let domain = String::from_utf8(raw_attr.value)?;
        if !domain.is_ascii() {
            return Err(crate::errors::StunMessageError::SyntaxError(format!(
                "domain name of {:?} attr is not an ascii string: {}",
                Self::STATIC_ATTR_TYPE.unwrap(),
                domain,
            )));
        }

        Ok(Self { domain })
    }

    fn into_raw_attr(
        self,
        _transaction_id: &crate::header::TransactionId,
    ) -> crate::attribute::RawAttribute {
        crate::attribute::RawAttribute::new(self.get_type(), self.domain.into_bytes())
    }
}
