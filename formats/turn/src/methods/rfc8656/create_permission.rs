use stun_formats::{
    MessageChecker, define_method,
    methods::{MethodExtDynamic, MethodExtDynamicInner, MethodExtStatic},
};

use crate::attributes::rfc8656;

define_method!(0x008, CREATE_PERMISSION, "CreatePermission");
impl MessageChecker for CREATE_PERMISSION {
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
        message.require_ext::<rfc8656::XorPeerAddressAttribute>()
    }
}
