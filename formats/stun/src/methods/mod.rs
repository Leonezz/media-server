use std::fmt;

use crate::{errors::STUNMessageResult, message::Message, methods::rfc8489::STUNMethodBinding};

pub mod rfc8489;
pub trait MethodExt {
    const VALUE: u16;
    fn get_name() -> &'static str;
    fn check(&self, message: &Message) -> STUNMessageResult<()>;
}

#[derive(Clone, Copy)]
pub enum Method {
    Binding(rfc8489::STUNMethodBinding),
    Reserved(u16),
}

impl fmt::Debug for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Binding(_) => f.write_str(STUNMethodBinding::get_name()),
            Self::Reserved(v) => write!(f, "Reserved({})", v),
        }
    }
}

impl From<u16> for Method {
    fn from(value: u16) -> Self {
        match value {
            rfc8489::STUNMethodBinding::VALUE => Self::Binding(STUNMethodBinding {}),
            _ => Self::Reserved(value),
        }
    }
}

impl From<Method> for u16 {
    fn from(val: Method) -> Self {
        match val {
            Method::Binding(_) => STUNMethodBinding::VALUE,
            Method::Reserved(v) => v,
        }
    }
}

impl Method {
    pub fn check(&self, message: &Message) -> STUNMessageResult<()> {
        match self {
            Self::Binding(b) => b.check(message),
            Self::Reserved(_) => Ok(()),
        }
    }
}
