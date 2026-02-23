#![feature(ip_as_octets)]
#![feature(buf_read_has_data_left)]
#![feature(error_generic_member_access)]
//! see: RFC 8489 Session Traversal Utilities for NAT (STUN)

use std::fmt::Debug;

use rootcause::bail;
use utils::errors::context::LocationExt;

use crate::errors::StunMessageResult;
pub mod attributes;
pub mod builder;
pub mod error_codes;
pub mod errors;
pub mod header;
pub mod message;
pub mod methods;
pub const STUN_URI_SCHEMA: &str = "stun";
pub const STUN_URI_SCHEMA_SECURE: &str = "stuns";
pub trait MessageChecker: Debug {
    #[allow(unused_variables)]
    fn allowed_in(&self, message_class: header::MessageClass) -> bool {
        // if a recognized attribute appears in an unexpected context (method or class), it is generally ignored by the receiving agent.
        true
    }
    fn check_message_class(&self, message: &message::Message) -> StunMessageResult<()> {
        if !self.allowed_in(message.message_class()) {
            bail!(crate::errors::StunMessageError::InvalidMessage(format!(
                "{:?} not allowed in {:?} class message",
                self,
                message.message_class()
            )));
        }
        Ok(())
    }
    #[allow(unused_variables)]
    fn check_request(&self, message: &message::Message) -> StunMessageResult<()> {
        Ok(())
    }
    #[allow(unused_variables)]
    fn check_success_response(&self, message: &message::Message) -> StunMessageResult<()> {
        Ok(())
    }
    #[allow(unused_variables)]
    fn check_error_response(&self, message: &message::Message) -> StunMessageResult<()> {
        Ok(())
    }
    #[allow(unused_variables)]
    fn check_indication(&self, message: &message::Message) -> StunMessageResult<()> {
        Ok(())
    }
    fn check(&self, message: &message::Message) -> StunMessageResult<()> {
        self.check_message_class(message).trace()?;
        match message.message_class() {
            header::MessageClass::Request => self.check_request(message).trace(),
            header::MessageClass::SuccessResponse => self.check_success_response(message).trace(),
            header::MessageClass::ErrorResponse => self.check_error_response(message).trace(),
            header::MessageClass::Indication => self.check_indication(message).trace(),
        }
    }
}
