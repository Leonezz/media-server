use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use num::ToPrimitive;
use rootcause::{Report, bail};
use std::{fmt, time::Duration};
use stun_formats::{
    MessageChecker,
    attributes::{AttributeExtDynamic, AttributeExtStatic, AttributeFactory},
    define_attribute,
};

// The LIFETIME attribute represents the duration for which the server
// will maintain an allocation in the absence of a refresh.
// The TURN client can include the LIFETIME attribute with the desired lifetime
// in Allocate and Refresh requests.
// The value portion of this attribute is 4 bytes long and consists of a
// 32-bit unsigned integral value representing the number of seconds remaining until expiration.
#[derive(Clone, Copy)]
pub struct LifeTimeAttribute {
    lifetime_seconds: u32,
}

impl LifeTimeAttribute {
    pub fn new(lifetime: Duration) -> Self {
        Self {
            lifetime_seconds: lifetime.as_secs().to_u32().unwrap(),
        }
    }

    pub fn lifetime(&self) -> Duration {
        Duration::from_secs(self.lifetime_seconds as u64)
    }
}

const LIFE_TIME_ATTR_LEN: usize = 4;

impl fmt::Debug for LifeTimeAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "lifetime: {}", self.lifetime_seconds)
    }
}

define_attribute!(0x000D, LifeTimeAttribute, "LIFETIME");

impl MessageChecker for LifeTimeAttribute {}

impl AttributeFactory for LifeTimeAttribute {
    fn from_raw_attr(
        raw_attr: stun_formats::attributes::RawAttribute,
        _transaction_id: &stun_formats::header::TransactionId,
    ) -> Result<Self, Report> {
        stun_formats::attributes::check_attr_match(raw_attr.attr_type, Self::STATIC_ATTR_TYPE)?;
        if raw_attr.value.len() != LIFE_TIME_ATTR_LEN {
            bail!(stun_formats::errors::StunMessageError::SyntaxError(
                format!(
                    "lifetime attribute expects {} bytes of value, got {} bytes instead",
                    LIFE_TIME_ATTR_LEN,
                    raw_attr.value.len()
                ),
            ))
        }
        let mut buffer = raw_attr.value.as_slice();
        let lifetime = buffer.read_u32::<BigEndian>()?;
        Ok(Self {
            lifetime_seconds: lifetime,
        })
    }

    fn into_raw_attr(
        self,
        _transaction_id: &stun_formats::header::TransactionId,
    ) -> stun_formats::attributes::RawAttribute {
        let mut value = Vec::with_capacity(LIFE_TIME_ATTR_LEN);
        value.write_u32::<BigEndian>(self.lifetime_seconds).unwrap();
        stun_formats::attributes::RawAttribute::new(Self::STATIC_ATTR_TYPE, value)
    }
}
