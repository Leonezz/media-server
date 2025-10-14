use std::fmt::Debug;

use crate::methods::MethodExt;

#[derive(Clone, Copy)]
pub struct STUNMethodBinding;

impl Debug for STUNMethodBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(STUNMethodBinding::static_name())
    }
}

impl MethodExt for STUNMethodBinding {
    const STATIC_VALUE: u16 = 0x001;
    fn static_name() -> &'static str {
        "Binding"
    }

    fn check(&self, message: &crate::message::Message) -> crate::errors::STUNMessageResult<()> {
        match message.message_class() {
            crate::header::MessageClass::ErrorResponse => {
                message.require(crate::attribute::AttrType::ErrorCode)
            }
            crate::header::MessageClass::SuccessResponse => {
                message.require(crate::attribute::AttrType::XorMappedAddress)
            }
            _ => Ok(()),
        }
    }
}
