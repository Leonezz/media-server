use std::fmt;

use utils::traits::dynamic_sized_packet::DynamicSizedPacket;

use crate::{
    attribute::STUNAttributeExt,
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
            return Err(crate::errors::STUNMessageError::SyntaxError(format!(
                "value length for {:?}: {} exceeds max length: {}",
                crate::attribute::AttrType::AlternateDomain,
                domain.len(),
                ALTERNATE_DOMAIN_MAX_LEN
            )));
        }

        if !domain.is_ascii() {
            return Err(crate::errors::STUNMessageError::SyntaxError(format!(
                "domain name of {:?} attr is not an ascii string: {}",
                crate::attribute::AttrType::AlternateDomain,
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

impl STUNAttributeExt for AlternateDomainAttribute {
    fn get_type(&self) -> crate::attribute::AttrType {
        crate::attribute::AttrType::AlternateDomain
    }

    fn from_raw_attr(
        raw_attr: crate::attribute::STUNRawAttribute,
        _transaction_id: &crate::header::TransactionId,
    ) -> Result<Self, crate::errors::STUNMessageError> {
        check_attr_match(
            raw_attr.attr_type,
            crate::attribute::AttrType::AlternateDomain,
        )?;
        if raw_attr.value.len() > ALTERNATE_DOMAIN_MAX_LEN {
            return Err(crate::errors::STUNMessageError::SyntaxError(format!(
                "value length for {:?}: {} exceeds max length: {}",
                crate::attribute::AttrType::AlternateDomain,
                raw_attr.value.len(),
                ALTERNATE_DOMAIN_MAX_LEN
            )));
        }
        let domain = String::from_utf8(raw_attr.value)?;
        if !domain.is_ascii() {
            return Err(crate::errors::STUNMessageError::SyntaxError(format!(
                "domain name of {:?} attr is not an ascii string: {}",
                crate::attribute::AttrType::AlternateDomain,
                domain,
            )));
        }

        Ok(Self { domain })
    }

    fn into_raw_attr(
        self,
        _transaction_id: &crate::header::TransactionId,
    ) -> crate::attribute::STUNRawAttribute {
        crate::attribute::STUNRawAttribute::new(self.get_type(), self.domain.into_bytes())
    }
}
