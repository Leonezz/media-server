use std::{collections::HashMap, io};

use errors::{AmfError, AmfResult};
use utils::traits::{
    reader::{ReadFrom, ReadRemainingFrom},
    writer::WriteTo,
};

pub mod amf0;
pub mod amf3;
pub mod errors;

#[derive(Debug, Clone)]
pub enum Value {
    AMF0Value(amf0::Value),
    AMF3Value(amf3::Value),
}

impl From<amf0::Value> for Value {
    fn from(value: amf0::Value) -> Self {
        Value::AMF0Value(value)
    }
}

impl From<amf3::Value> for Value {
    fn from(value: amf3::Value) -> Self {
        Value::AMF3Value(value)
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Version {
    #[default]
    Amf0 = 0,
    Amf3 = 3,
}

impl<R: io::Read> ReadRemainingFrom<Version, R> for Value {
    type Error = AmfError;
    fn read_remaining_from(header: Version, reader: &mut R) -> Result<Self, Self::Error> {
        match header {
            Version::Amf0 => amf0::Value::read_from(reader).map(Value::AMF0Value),
            Version::Amf3 => amf3::Value::read_from(reader).map(Value::AMF3Value),
        }
    }
}

impl Value {
    pub fn read_string<R: io::Read>(reader: &mut R, version: Version) -> AmfResult<Option<String>> {
        let value = Value::read_remaining_from(version, reader)?;
        Ok(value.try_as_str().map(|v| v.to_owned()))
    }

    pub fn read_null<R: io::Read>(reader: &mut R, version: Version) -> AmfResult<Option<()>> {
        let value = Value::read_remaining_from(version, reader)?;
        match value {
            Value::AMF0Value(value) => {
                if matches!(value, amf0::Value::Null) {
                    Ok(Some(()))
                } else {
                    Ok(None)
                }
            }
            Value::AMF3Value(value) => {
                if matches!(value, amf3::Value::Null) {
                    Ok(Some(()))
                } else {
                    Ok(None)
                }
            }
        }
    }

    pub fn read_number<R: io::Read>(reader: &mut R, version: Version) -> AmfResult<Option<f64>> {
        let value = Value::read_remaining_from(version, reader)?;
        Ok(value.try_as_f64())
    }

    pub fn read_object<R: io::Read>(
        reader: &mut R,
        version: Version,
    ) -> AmfResult<Option<HashMap<String, Value>>> {
        let value = Value::read_remaining_from(version, reader)?;
        match value.try_into_pairs() {
            Ok(iter) => Ok(Some(iter.collect::<HashMap<String, Value>>())),
            _ => Ok(None),
        }
    }

    pub fn read_bool<R: io::Read>(reader: &mut R, version: Version) -> AmfResult<Option<bool>> {
        let value = Value::read_remaining_from(version, reader)?;
        Ok(value.try_as_bool())
    }

    pub fn read_all<R>(reader: R, version: Version) -> AmfResult<Vec<Self>>
    where
        R: io::Read,
    {
        match version {
            Version::Amf0 => Ok(amf0::Value::read_all(reader)?
                .iter()
                .map(|v| Value::from(v.clone()))
                .collect()),
            Version::Amf3 => Ok(amf3::Value::read_all(reader)?
                .iter()
                .map(|v| Value::from(v.clone()))
                .collect()),
        }
    }

    pub fn write_str<W>(value: &str, writer: &mut W, version: Version) -> AmfResult<()>
    where
        W: io::Write,
    {
        let value = match version {
            Version::Amf0 => Value::AMF0Value(amf0::Value::String(value.to_string())),
            Version::Amf3 => Value::AMF3Value(amf3::Value::String(value.to_string())),
        };
        Value::write_to(&value, writer)
    }

    pub fn write_bool<W>(value: bool, writer: &mut W, version: Version) -> AmfResult<()>
    where
        W: io::Write,
    {
        let value = match version {
            Version::Amf0 => Value::AMF0Value(amf0::Value::Boolean(value)),
            Version::Amf3 => Value::AMF3Value(amf3::Value::Boolean(value)),
        };
        Value::write_to(&value, writer)
    }

    pub fn write_number<W: io::Write, T: Into<f64>>(
        value: T,
        writer: &mut W,
        version: Version,
    ) -> AmfResult<()> {
        let value = match version {
            Version::Amf0 => Value::AMF0Value(amf0::Value::Number(value.into())),
            Version::Amf3 => Value::AMF3Value(amf3::Value::Double(value.into())),
        };
        Value::write_to(&value, writer)
    }

    pub fn write_null<W>(writer: &mut W, version: Version) -> AmfResult<()>
    where
        W: io::Write,
    {
        match version {
            Version::Amf0 => Value::write_to(&Value::AMF0Value(amf0::Value::Null), writer),
            Version::Amf3 => Value::write_to(&Value::AMF3Value(amf3::Value::Null), writer),
        }
    }

    pub fn write_nullable_object<W: io::Write, T: Into<HashMap<String, Value>>>(
        value: Option<T>,
        writer: &mut W,
        version: Version,
    ) -> AmfResult<()> {
        match value {
            Some(obj) => Self::write_key_value_pairs(obj.into(), writer, version)?,
            None => Self::write_null(writer, version)?,
        }
        Ok(())
    }

    pub fn write_key_value_pairs<W>(
        value: HashMap<String, Value>,
        writer: &mut W,
        version: Version,
    ) -> AmfResult<()>
    where
        W: io::Write,
    {
        if version == Version::Amf0 {
            // we can write any value with amf0
            let mut pairs: Vec<(String, amf0::Value)> = Vec::new();
            for (k, v) in value {
                match v {
                    Value::AMF0Value(v0_value) => pairs.push((k, v0_value)),
                    Value::AMF3Value(v3_value) => pairs.push((k, amf0::Value::AVMPlus(v3_value))),
                }
            }

            Value::write_to(
                &Value::AMF0Value(amf0::Value::Object {
                    name: None,
                    entries: pairs,
                }),
                writer,
            )
        } else {
            let mut all_v3 = true;
            for v in value.values() {
                if let Value::AMF0Value(_) = v {
                    all_v3 = false;
                }
            }

            let mut pairs: Vec<(String, amf3::Value)> = Vec::new();
            for (k, v) in &value {
                if let Value::AMF3Value(v3_value) = v {
                    pairs.push((k.clone(), v3_value.clone()));
                } else {
                    unreachable!("object should be full of v3 value");
                }
            }

            if all_v3 {
                // if all values are amf3, amf3 could work
                Value::write_to(
                    &Value::AMF3Value(amf3::Value::Object {
                        name: None,
                        sealed_fields_count: 0,
                        entries: pairs,
                    }),
                    writer,
                )
            } else {
                // otherwise we go back to amf0
                Value::write_key_value_pairs(value, writer, Version::Amf0)
            }
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

    pub fn try_as_bool(&self) -> Option<bool> {
        match *self {
            Value::AMF0Value(ref v) => v.try_as_bool(),
            Value::AMF3Value(ref v) => v.try_as_bool(),
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

fn iter_boxed<I, T>(iter: I) -> Box<dyn Iterator<Item = T>>
where
    I: Iterator<Item = T> + 'static,
{
    Box::new(iter)
}
/// Makes a `String` value.
pub fn string<T>(t: T, version: Version) -> Value
where
    String: From<T>,
{
    match version {
        Version::Amf0 => Value::AMF0Value(amf0::string(t)),
        Version::Amf3 => Value::AMF3Value(amf3::string(t)),
    }
}

/// Makes a `Number` value.
pub fn number<T>(t: T, version: Version) -> Value
where
    f64: From<T>,
{
    match version {
        Version::Amf0 => Value::AMF0Value(amf0::number(t)),
        Version::Amf3 => Value::AMF3Value(amf3::number(t)),
    }
}
/// Makes a `Bool` value.
pub fn bool<T>(t: T, version: Version) -> Value
where
    bool: From<T>,
{
    match version {
        Version::Amf0 => Value::AMF0Value(amf0::bool(t)),
        Version::Amf3 => Value::AMF3Value(amf3::bool(t)),
    }
}

pub trait AmfComplexObject {
    fn extract_bool_field(&self, key: &str) -> Option<bool>;
    fn extract_number_field(&self, key: &str) -> Option<f64>;
    fn extract_string_field(&self, key: &str) -> Option<String>;
    fn extract_array_field(&self, key: &str) -> Option<Box<dyn Iterator<Item = Value>>>;
    fn extract_object_field(&self, key: &str) -> Option<Box<dyn Iterator<Item = (String, Value)>>>;
}

impl AmfComplexObject for HashMap<String, Value> {
    fn extract_bool_field(&self, key: &str) -> Option<bool> {
        match self.get(key) {
            Some(value) => value.try_as_bool(),
            None => None,
        }
    }

    fn extract_number_field(&self, key: &str) -> Option<f64> {
        match self.get(key) {
            Some(value) => value.try_as_f64(),
            None => None,
        }
    }

    fn extract_string_field(&self, key: &str) -> Option<String> {
        match self.get(key) {
            Some(value) => value.try_as_str().map(|s| s.to_string()),
            None => None,
        }
    }

    fn extract_array_field(&self, key: &str) -> Option<Box<dyn Iterator<Item = Value>>> {
        match self.get(key).cloned() {
            Some(v) => v.try_into_values().ok(),
            None => None,
        }
    }

    fn extract_object_field(&self, key: &str) -> Option<Box<dyn Iterator<Item = (String, Value)>>> {
        match self.get(key).cloned() {
            Some(v) => v.try_into_pairs().ok(),
            None => None,
        }
    }
}

impl<W: io::Write> WriteTo<W> for Value {
    type Error = AmfError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        match self {
            Value::AMF0Value(v) => v.write_to(writer),
            Value::AMF3Value(v) => v.write_to(writer),
        }
    }
}
