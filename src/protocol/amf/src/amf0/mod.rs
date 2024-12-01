//!@see: [Action Message Format -- AMF 0](https://rtmp.veriskope.com/pdf/amf0-file-format-specification.pdf).

use core::time;
use std::io;

pub use self::decode::Decoder;
pub use self::encode::Encoder;
use crate::amf3;
use crate::error::{AmfDecodeResult, AmfEncodeResult};

mod decode;
mod encode;

/// @see: 2.1 Types Overview
mod amf0_marker {
    pub const NUMBER: u8 = 0x00;
    pub const BOOLEAN: u8 = 0x01;
    pub const STRING: u8 = 0x02;
    pub const OBJECT: u8 = 0x03;
    pub const MOVIECLIP: u8 = 0x04; // reserved, not supported
    pub const NULL: u8 = 0x05;
    pub const UNDEFINED: u8 = 0x06;
    pub const REFERENCE: u8 = 0x07;
    pub const ECMA_ARRAY: u8 = 0x08;
    pub const OBJECT_END: u8 = 0x09;
    pub const STRICT_ARRAY: u8 = 0x0a;
    pub const DATE: u8 = 0x0b;
    pub const LONG_STRING: u8 = 0x0c;
    pub const UNSUPPORTED: u8 = 0x0d;
    pub const RECORDSET: u8 = 0x0e; // reserved, not supported
    pub const XML_DOCUMENT: u8 = 0x0f;
    pub const TYPED_OBJECT: u8 = 0x10;
    pub const AVMPLUS_OBJECT: u8 = 0x11;
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// @see: 2.2 Number Type
    Number(f64),
    /// @see: 2.3 Boolean Type
    Boolean(bool),
    /// @see: 2.4 String Type
    /// @see: 2.14 Long String Type
    String(String),
    /// @see: 2.5 Object Type, 2.18 Typed Object Type
    Object {
        name: Option<String>,
        entries: Vec<(String, Value)>,
    },
    /// @see: 2.7 null Type
    Null,
    /// @see: 2.8 undefined Type
    Undefined,
    /// @see: 2.9 Reference Type
    Reference { index: u16 },
    /// @see: 2.10 ECMA Array Type
    ECMAArray(Vec<(String, Value)>),
    /// @see: 2.11 Object End Type
    ObjectEnd,
    /// @see: 2.12 Strict Array Type
    StrictArray(Vec<Value>),
    // @see: 2.13 Date Type
    Date {
        ///NOTE - this is reserved, should always be 0x0000, otherwise it will be unexpected
        time_zone: i16,
        millis_timestamp: time::Duration,
    },
    /// @see: 2.17 XML Document Type
    XMLDocument(String),
    /// @see: 3.1 AVM+ Type Marker
    AVMPlus(amf3::Value),
}

impl Value {
    pub fn read_from<R>(reader: R) -> AmfDecodeResult<Self>
    where
        R: io::Read,
    {
        Decoder::new(reader).decode()
    }

    pub fn write_to<W>(&self, writer: W) -> AmfEncodeResult<()>
    where
        W: io::Write,
    {
        Encoder::new(writer).encode(self)
    }

    pub fn try_as_str(&self) -> Option<&str> {
        match *self {
            Value::String(ref str) => Some(str),
            Value::XMLDocument(ref str) => Some(str),
            Value::AVMPlus(ref v) => v.try_as_str(),
            _ => None,
        }
    }

    pub fn try_as_f64(&self) -> Option<f64> {
        match *self {
            Value::Number(v) => Some(v),
            Value::AVMPlus(ref v) => v.try_as_f64(),
            _ => None,
        }
    }

    pub fn try_into_values(self) -> Result<Box<dyn Iterator<Item = super::Value>>, Self> {
        match self {
            Value::StrictArray(arr) => Ok(Box::new(arr.into_iter().map(super::Value::AMF0Value))),
            Value::AVMPlus(v) => v
                .try_into_values()
                .map(|iter| iter.map(super::Value::AMF3Value))
                .map(super::iter_boxed)
                .map_err(Value::AVMPlus),
            _ => Err(self),
        }
    }

    pub fn try_into_pairs(self) -> Result<Box<dyn Iterator<Item = (String, super::Value)>>, Self> {
        match self {
            Value::ECMAArray(arr) => Ok(Box::new(
                arr.into_iter()
                    .map(|(key, value)| (key, super::Value::AMF0Value(value))),
            )),
            Value::Object { entries, .. } => Ok(Box::new(
                entries
                    .into_iter()
                    .map(|(key, value)| (key, super::Value::AMF0Value(value))),
            )),
            Value::AVMPlus(v) => v
                .try_into_pairs()
                .map(|iter| iter.map(|(key, value)| (key, super::Value::AMF3Value(value))))
                .map(super::iter_boxed)
                .map_err(Value::AVMPlus),
            _ => Err(self),
        }
    }
}

/// Makes a `String` value.
pub fn string<T>(t: T) -> Value
where
    String: From<T>,
{
    Value::String(From::from(t))
}

/// Makes a `Number` value.
pub fn number<T>(t: T) -> Value
where
    f64: From<T>,
{
    Value::Number(From::from(t))
}

/// Makes an anonymous `Object` value.
pub fn object<I, K>(entries: I) -> Value
where
    I: Iterator<Item = (K, Value)>,
    String: From<K>,
{
    Value::Object {
        name: None,
        entries: entries.map(|(k, v)| (From::from(k), v)).collect(),
    }
}

/// Make a strict `Array` value.
pub fn array(entries: Vec<Value>) -> Value {
    Value::StrictArray(entries)
}
