use stun_formats::{
    MessageChecker,
    attributes::AttributeExtStatic,
    define_method,
    methods::{MethodExtDynamic, MethodExtDynamicInner, MethodExtStatic},
};

use crate::attributes::rfc8656;

define_method!(0x009, CHANNEL_BIND, "ChannelBind");

impl MessageChecker for CHANNEL_BIND {
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
        message.require_ext::<rfc8656::ChannelNumberAttribute>()?;
        let channel_number = message
            .get_attribute_ext::<rfc8656::ChannelNumberAttribute>()
            .unwrap()
            .channel_number;
        if channel_number < 0x4000 || channel_number > 0x4FFF {
            return Err(stun_formats::errors::StunMessageError::InvalidMessage(
                format!(
                    "{} should be in [0x4000, 0x4FFF]",
                    rfc8656::ChannelNumberAttribute::STATIC_NAME
                ),
            ));
        }
        message.require_ext::<rfc8656::XorPeerAddressAttribute>()?;
        Ok(())
    }
}
