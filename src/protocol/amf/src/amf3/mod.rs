///! @see: [Action Message Format -- AMF 3](https://rtmp.veriskope.com/pdf/amf3-file-format-spec.pdf)
use core::time;
use std::io::{self};

mod decode;
mod encode;

use crate::error::{AmfDecodeResult, AmfEncodeResult};

pub use self::decode::Decoder;
pub use self::encode::Encoder;

/// @see: 3.1 Overview
mod amf3_marker {
    pub const UNDEFINED: u8 = 0x00;
    pub const NULL: u8 = 0x01;
    pub const FALSE: u8 = 0x02;
    pub const TRUE: u8 = 0x03;
    pub const INTEGER: u8 = 0x04;
    pub const DOUBLE: u8 = 0x05;
    pub const STRING: u8 = 0x06;
    pub const XML_DOCUMENT: u8 = 0x07;
    pub const DATE: u8 = 0x08;
    pub const ARRAY: u8 = 0x09;
    pub const OBJECT: u8 = 0x0A;
    pub const XML: u8 = 0x0B;
    pub const BYTE_ARRAY: u8 = 0x0C;
    pub const VECTOR_INT: u8 = 0x0D;
    pub const VECTOR_UINT: u8 = 0x0E;
    pub const VECTOR_DOUBLE: u8 = 0x0F;
    pub const VECTOR_OBJECT: u8 = 0x10;
    pub const DICTIONARY: u8 = 0x11;
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// @see: 3.2 undefined Type
    Undefined,
    /// @see: 3.3 null Type
    Null,
    /// @see: 3.4 false Type, 3.5 true type
    Boolean(bool),
    /// @see: 3.6 integer type
    Integer(i32),
    /// @see: 3.7 double type
    Double(f64),
    /// @see: 3.8 String type
    String(String),
    /// @see: 3.9 XMLDocument type
    XMLDocument(String),
    /// @see: 3.10 Date type
    Date {
        millis_timestamp: time::Duration,
    },
    /// @see: 3.11 Array type
    Array {
        assoc_entries: Vec<(String, Value)>,
        dense_entries: Vec<Value>,
    },
    /// @see: 3.12 Object type
    Object {
        name: Option<String>,
        sealed_fields_count: usize,
        entries: Vec<(String, Value)>,
    },
    /// @see: 3.13 XML type
    XML(String),
    /// @see: 3.14 ByteArray type
    ByteArray(Vec<u8>),
    /// @see: 3.15 Vector Type
    I32Vector {
        is_fixed: bool,
        entries: Vec<i32>,
    },
    U32Vector {
        is_fixed: bool,
        entries: Vec<u32>,
    },
    DoubleVector {
        is_fixed: bool,
        entries: Vec<f64>,
    },
    ObjectVector {
        is_fixed: bool,
        class_name: Option<String>,
        entries: Vec<Value>,
    },

    /// @see: 3.16 Dictionary Type
    Dictionary {
        is_weak: bool,
        entries: Vec<(Value, Value)>,
    },
}

#[derive(Debug, Clone)]
pub struct Amf3Trait {
    class_name: Option<String>,
    is_dynamic: bool,
    fields: Vec<String>,
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
            Value::XML(ref str) => Some(str),
            _ => None,
        }
    }

    pub fn try_as_f64(&self) -> Option<f64> {
        match *self {
            Value::Integer(v) => Some(v as f64),
            Value::Double(v) => Some(v),
            _ => None,
        }
    }

    pub fn try_into_values(self) -> Result<Box<dyn Iterator<Item = Value>>, Self> {
        match self {
            Value::Array { dense_entries, .. } => Ok(Box::new(dense_entries.into_iter())),
            Value::I32Vector { entries, .. } => {
                Ok(Box::new(entries.into_iter().map(Value::Integer)))
            }
            Value::U32Vector { entries, .. } => Ok(Box::new(
                entries.into_iter().map(|v| Value::Double(v as f64)),
            )),
            Value::DoubleVector { entries, .. } => {
                Ok(Box::new(entries.into_iter().map(Value::Double)))
            }
            Value::ObjectVector { entries, .. } => Ok(Box::new(entries.into_iter())),
            _ => Err(self),
        }
    }

    pub fn try_into_pairs(self) -> Result<Box<dyn Iterator<Item = (String, Value)>>, Self> {
        match self {
            Value::Array { assoc_entries, .. } => Ok(Box::new(assoc_entries.into_iter())),
            Value::Object { entries, .. } => Ok(Box::new(entries.into_iter())),
            _ => Err(self),
        }
    }
}
