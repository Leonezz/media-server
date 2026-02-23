use rootcause::{Report, bail};
use std::fmt;
use stun_formats::{
    MessageChecker,
    attributes::{AttributeExtDynamic, AttributeExtStatic, AttributeFactory},
    define_attribute,
};

// The RESERVATION-TOKEN attribute contains a token that uniquely
// identifies a relayed transport address being held in reserve by the server.
// The server includes this attribute in a success response to tell the client about the token,
// and the client includes this attribute in a subsequent Allocate request to request
// the server use that relayed transport address for the allocation.
// The attribute value is 8 bytes and contains the token value.
#[derive(Clone, Copy)]
pub struct ReservationTokenAttribute {
    token: [u8; RESERVATION_TOKEN_ATTR_LEN],
}

impl ReservationTokenAttribute {
    pub fn new(token: [u8; RESERVATION_TOKEN_ATTR_LEN]) -> Self {
        Self { token }
    }
    pub fn new_random() -> Self {
        let mut token = [0_u8; RESERVATION_TOKEN_ATTR_LEN];
        utils::random::random_fill(&mut token);
        Self { token }
    }
    pub fn token(&self) -> &[u8; RESERVATION_TOKEN_ATTR_LEN] {
        &self.token
    }
}

const RESERVATION_TOKEN_ATTR_LEN: usize = 8;

impl fmt::Display for ReservationTokenAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "reservation token: 0x{:x?}", self.token)
    }
}

impl fmt::Debug for ReservationTokenAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

define_attribute!(0x0022, ReservationTokenAttribute, "RESERVATION_TOKEN");

impl MessageChecker for ReservationTokenAttribute {}

impl AttributeFactory for ReservationTokenAttribute {
    fn from_raw_attr(
        raw_attr: stun_formats::attributes::RawAttribute,
        _transaction_id: &stun_formats::header::TransactionId,
    ) -> Result<Self, Report> {
        stun_formats::attributes::check_attr_match(raw_attr.attr_type, Self::STATIC_ATTR_TYPE)?;
        if raw_attr.value.len() != RESERVATION_TOKEN_ATTR_LEN {
            bail!(stun_formats::errors::StunMessageError::SyntaxError(
                format!(
                    "reservation token attribute expects {} bytes, got {} bytes instead",
                    RESERVATION_TOKEN_ATTR_LEN,
                    raw_attr.value.len()
                ),
            ))
        }
        let token: [u8; 8] = raw_attr.value.try_into().unwrap();
        Ok(Self { token })
    }

    fn into_raw_attr(
        self,
        _transaction_id: &stun_formats::header::TransactionId,
    ) -> stun_formats::attributes::RawAttribute {
        stun_formats::attributes::RawAttribute::new(Self::STATIC_ATTR_TYPE, self.token.into())
    }
}
