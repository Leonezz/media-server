use stun_formats::{
    MessageChecker, define_method,
    methods::{MethodExtDynamic, MethodExtDynamicInner, MethodExtStatic},
};

define_method!(0x006, SEND, "Send");

impl MessageChecker for SEND {
    fn allowed_in(&self, message_class: stun_formats::header::MessageClass) -> bool {
        matches!(
            message_class,
            stun_formats::header::MessageClass::Indication
        )
    }
}
