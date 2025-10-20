use stun_formats::{
    MessageChecker, define_method,
    methods::{MethodExtDynamic, MethodExtDynamicInner, MethodExtStatic},
};

define_method!(0x004, REFRESH, "Refresh");
impl MessageChecker for REFRESH {
    fn allowed_in(&self, message_class: stun_formats::header::MessageClass) -> bool {
        !matches!(
            message_class,
            stun_formats::header::MessageClass::Indication
        )
    }
}
