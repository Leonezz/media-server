use super::MethodExtStatic;
use crate::{
    MessageChecker,
    attributes::rfc8489,
    define_method,
    methods::{MethodExtDynamic, MethodExtDynamicInner},
};

define_method!(0x001, BINDING, "Binding");

impl MessageChecker for BINDING {
    fn allowed_in(&self, message_class: crate::header::MessageClass) -> bool {
        !matches!(message_class, crate::header::MessageClass::Indication)
    }
    fn check_success_response(
        &self,
        message: &crate::message::Message,
    ) -> crate::errors::StunMessageResult<()> {
        message.require_ext::<rfc8489::XorMappedAddressAttribute>()
    }
}

impl MethodExtDynamic for BINDING {}
