use std::fmt::Debug;

use crate::methods::STUNMethodExt;

#[derive(Clone, Copy)]
pub struct STUNMethodBinding;

impl Debug for STUNMethodBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(STUNMethodBinding::get_name())
    }
}

impl STUNMethodExt for STUNMethodBinding {
    const VALUE: u16 = 0x001;
    fn get_name() -> &'static str {
        "Binding"
    }

    fn check(&self, message: &crate::message::STUNMessage) -> crate::errors::STUNMessageResult<()> {
        match message.message_class() {
            crate::header::STUNMessageClass::ErrorResponse => {
                message.require(crate::attribute::AttrType::ErrorCode)
            }
            crate::header::STUNMessageClass::SuccessResponse => {
                message.require(crate::attribute::AttrType::XorMappedAddress)
            }
            _ => Ok(()),
        }
    }
}
