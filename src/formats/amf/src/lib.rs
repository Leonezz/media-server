use std::io;

use errors::{AmfReadResult, AmfWriteResult};

pub mod amf0;
pub mod amf3;
pub mod errors;

#[derive(Debug, Clone)]
pub enum Value {
    AMF0Value(amf0::Value),
    AMF3Value(amf3::Value),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Version {
    Amf0,
    Amf3,
}

impl Value {
    pub fn read_from<R>(reader: R, version: Version) -> AmfReadResult<Self>
    where
        R: io::Read,
    {
        match version {
            Version::Amf0 => amf0::Value::read_from(reader).map(Value::AMF0Value),
            Version::Amf3 => amf3::Value::read_from(reader).map(Value::AMF3Value),
        }
    }

    pub fn write_to<W>(&self, writer: W) -> AmfWriteResult
    where
        W: io::Write,
    {
        match *self {
            Value::AMF0Value(ref v) => v.write_to(writer),
            Value::AMF3Value(ref v) => v.write_to(writer),
        }
    }

    pub fn try_as_str(&self) -> Option<&str> {
        match *self {
            Value::AMF0Value(ref v) => v.try_as_str(),
            Value::AMF3Value(ref v) => v.try_as_str(),
        }
    }

    pub fn try_as_f64(&self) -> Option<f64> {
        match *self {
            Value::AMF0Value(ref v) => v.try_as_f64(),
            Value::AMF3Value(ref v) => v.try_as_f64(),
        }
    }

    pub fn try_into_values(self) -> Result<Box<dyn Iterator<Item = Value>>, Self> {
        match self {
            Value::AMF0Value(v) => v.try_into_values().map_err(Value::AMF0Value),
            Value::AMF3Value(v) => v
                .try_into_values()
                .map(|iter| iter.map(Value::AMF3Value))
                .map(iter_boxed)
                .map_err(Value::AMF3Value),
        }
    }

    pub fn try_into_pairs(self) -> Result<Box<dyn Iterator<Item = (String, Value)>>, Self> {
        match self {
            Value::AMF0Value(v) => v.try_into_pairs().map_err(Value::AMF0Value),
            Value::AMF3Value(v) => v
                .try_into_pairs()
                .map(|iter| iter.map(|(key, value)| (key, Value::AMF3Value(value))))
                .map(iter_boxed)
                .map_err(Value::AMF3Value),
        }
    }
}

impl From<amf0::Value> for Value {
    fn from(v: amf0::Value) -> Value {
        Value::AMF0Value(v)
    }
}
impl From<amf3::Value> for Value {
    fn from(v: amf3::Value) -> Value {
        Value::AMF3Value(v)
    }
}

fn iter_boxed<I, T>(iter: I) -> Box<dyn Iterator<Item = T>>
where
    I: Iterator<Item = T> + 'static,
{
    Box::new(iter)
}
