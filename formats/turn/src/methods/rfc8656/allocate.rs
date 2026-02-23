use crate::attributes::rfc8656;
use rootcause::bail;
use stun_formats::{
    MessageChecker,
    attributes::{AttributeExtStatic, rfc8489},
    define_method,
    methods::{MethodExtDynamic, MethodExtDynamicInner, MethodExtStatic},
};
define_method!(0x003, ALLOCATE, "Allocate");

impl MessageChecker for ALLOCATE {
    fn allowed_in(&self, message_class: stun_formats::header::MessageClass) -> bool {
        !matches!(
            message_class,
            stun_formats::header::MessageClass::Indication
        )
    }

    fn check_request(
        &self,
        message: &stun_formats::message::Message,
    ) -> stun_formats::errors::StunMessageResult<()> {
        message.require_ext::<rfc8656::RequestedTransportAttribute>()?;
        if message
            .get_attribute(rfc8656::ReservationTokenAttribute::STATIC_ATTR_TYPE)
            .is_some_and(|_| {
                message
                    .get_attribute(rfc8656::EvenPortAttribute::STATIC_ATTR_TYPE)
                    .is_some()
                    || message
                        .get_attribute(rfc8656::RequestedAddressFamilyAttribute::STATIC_ATTR_TYPE)
                        .is_some()
                    || message
                        .get_attribute(rfc8656::AdditionalAddressFamilyAttribute::STATIC_ATTR_TYPE)
                        .is_some()
            })
        {
            bail!(stun_formats::errors::StunMessageError::InvalidMessage(
                format!(
                    "{} request with {} attribute cannot have {}, {} or {} also",
                    Self::STATIC_NAME,
                    rfc8656::ReservationTokenAttribute::STATIC_NAME,
                    rfc8656::EvenPortAttribute::STATIC_NAME,
                    rfc8656::RequestedAddressFamilyAttribute::STATIC_NAME,
                    rfc8656::AdditionalAddressFamilyAttribute::STATIC_NAME
                ),
            ))
        }
        Ok(())
    }

    fn check_success_response(
        &self,
        message: &stun_formats::message::Message,
    ) -> stun_formats::errors::StunMessageResult<()> {
        message.require_ext::<rfc8656::XorRelayedAddressAttribute>()?;
        message.require_ext::<rfc8656::LifeTimeAttribute>()?;
        message.require_ext::<rfc8489::XorMappedAddressAttribute>()?;
        Ok(())
    }
}
