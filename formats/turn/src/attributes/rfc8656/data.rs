use rootcause::Report;
use std::fmt;
use stun_formats::{
    MessageChecker,
    attributes::{AttributeExtDynamic, AttributeExtStatic, AttributeFactory},
    define_attribute,
};

// The DATA attribute is present in all Send indications.
// If the ICMP attribute is not present in a Data indication,
// it contains a DATA attribute.
// The value portion of this attribute is variable length and
// consists of the application data (that is, the data that would
// immediately follow the UDP header if the data was sent directly
// between the client and the peer).
// The application data is equivalent to the "UDP user data" and does not
// include the "surplus area" defined in .
// If the length of this attribute is not a multiple of 4, then padding must be added after this attribute.
#[derive(Clone)]
pub struct DataAttribute {
    data: Vec<u8>,
}

impl DataAttribute {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn data(self) -> Vec<u8> {
        self.data
    }
}

impl fmt::Debug for DataAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "data: {} bytes", self.data.len())
    }
}

define_attribute!(0x0013, DataAttribute, "DATA");

impl MessageChecker for DataAttribute {}

impl AttributeFactory for DataAttribute {
    fn from_raw_attr(
        raw_attr: stun_formats::attributes::RawAttribute,
        _transaction_id: &stun_formats::header::TransactionId,
    ) -> Result<Self, Report> {
        stun_formats::attributes::check_attr_match(raw_attr.attr_type, Self::STATIC_ATTR_TYPE)?;
        Ok(Self {
            data: raw_attr.value,
        })
    }

    fn into_raw_attr(
        self,
        _transaction_id: &stun_formats::header::TransactionId,
    ) -> stun_formats::attributes::RawAttribute {
        stun_formats::attributes::RawAttribute::new(Self::STATIC_ATTR_TYPE, self.data)
    }
}
