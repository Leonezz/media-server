use std::fmt;

use crate::{errors::STUNMessageResult, message::STUNMessage, methods::rfc8489::STUNMethodBinding};

pub mod rfc8489;
pub trait STUNMethodExt {
    const VALUE: u16;
    fn get_name() -> &'static str;
    fn check(&self, message: &STUNMessage) -> STUNMessageResult<()>;
}

#[derive(Clone, Copy)]
pub enum STUNMethod {
    Binding(rfc8489::STUNMethodBinding),
    Reserved(u16),
}

impl fmt::Debug for STUNMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Binding(_) => f.write_str(STUNMethodBinding::get_name()),
            Self::Reserved(v) => write!(f, "Reserved({})", v),
        }
    }
}

impl From<u16> for STUNMethod {
    fn from(value: u16) -> Self {
        match value {
            rfc8489::STUNMethodBinding::VALUE => Self::Binding(STUNMethodBinding {}),
            _ => Self::Reserved(value),
        }
    }
}

impl From<STUNMethod> for u16 {
    fn from(val: STUNMethod) -> Self {
        match val {
            STUNMethod::Binding(_) => STUNMethodBinding::VALUE,
            STUNMethod::Reserved(v) => v,
        }
    }
}

impl STUNMethod {
    pub fn check(&self, message: &STUNMessage) -> STUNMessageResult<()> {
        match self {
            Self::Binding(b) => b.check(message),
            Self::Reserved(_) => Ok(()),
        }
    }
}
