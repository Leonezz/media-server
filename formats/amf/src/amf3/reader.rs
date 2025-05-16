use core::time;
use std::io;

use crate::errors::{AmfError, AmfResult};
use byteorder::{BigEndian, ReadBytesExt};
use utils::traits::reader::ReadFrom;

use super::{Amf3Trait, Value, amf3_marker};

enum SizeOrIndex {
    Size(usize),
    Index(usize),
}

#[derive(Debug)]
struct Amf3Referenceable {
    traits: Vec<Amf3Trait>,
    strings: Vec<String>,
    objects: Vec<Value>,
}

#[derive(Debug)]
pub struct Reader<R> {
    inner: R,
    referenceable: Amf3Referenceable,
}

impl<R> Reader<R> {
    pub fn into_inner(self) -> R {
        self.inner
    }
    pub fn inner(&mut self) -> &R {
        &self.inner
    }
    pub fn inner_mut(&mut self) -> &mut R {
        &mut self.inner
    }
}

impl<R> Reader<R>
where
    R: io::Read,
{
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            referenceable: Amf3Referenceable {
                traits: Vec::new(),
                strings: Vec::new(),
                objects: Vec::new(),
            },
        }
    }
    pub fn read(&mut self) -> AmfResult<Value> {
        let marker = self.inner.read_u8()?;

        match marker {
            amf3_marker::UNDEFINED => Ok(Value::Undefined),
            amf3_marker::NULL => Ok(Value::Null),
            amf3_marker::FALSE => Ok(Value::Boolean(false)),
            amf3_marker::TRUE => Ok(Value::Boolean(true)),
            amf3_marker::INTEGER => self.read_integer(),
            amf3_marker::DOUBLE => self.read_double(),
            amf3_marker::STRING => self.read_string(),
            amf3_marker::XML_DOCUMENT => self.read_xml_document(),
            amf3_marker::DATE => self.read_date(),
            amf3_marker::ARRAY => self.read_array(),
            amf3_marker::OBJECT => self.read_object(),
            amf3_marker::XML => self.read_xml(),
            amf3_marker::BYTE_ARRAY => self.read_byte_array(),
            amf3_marker::VECTOR_INT => self.read_i32_vector(),
            amf3_marker::VECTOR_UINT => self.read_u32_vector(),
            amf3_marker::VECTOR_DOUBLE => self.read_double_vector(),
            amf3_marker::VECTOR_OBJECT => self.read_object_vector(),
            amf3_marker::DICTIONARY => self.read_dictionary(),
            _ => Err(AmfError::Unknown { marker }),
        }
    }

    pub fn read_all(&mut self) -> AmfResult<Vec<Value>> {
        let mut result = Vec::new();
        while let Ok(value) = self.read() {
            result.push(value);
        }
        Ok(result)
    }

    fn read_u29(&mut self) -> AmfResult<u32> {
        let mut result: u32 = 0;
        for _ in 0..3 {
            let byte = self.inner.read_u8()?;
            result = (result << 7) | ((byte as u32) & 0b0111_1111);
            if (byte & 0b1000_0000) == 0 {
                return Ok(result);
            }
        }
        let byte = self.inner.read_u8()?;
        Ok((result << 8) | (byte as u32))
    }
    fn read_size_or_index(&mut self) -> AmfResult<SizeOrIndex> {
        let u29 = self.read_u29()? as usize;
        let is_index = (u29 & 0b01) == 0;
        let value = u29 >> 1;
        if is_index {
            Ok(SizeOrIndex::Index(value))
        } else {
            Ok(SizeOrIndex::Size(value))
        }
    }
    fn read_bytes(&mut self, len: usize) -> AmfResult<Vec<u8>> {
        let mut buf = vec![0; len];
        self.inner.read_exact(&mut buf)?;
        Ok(buf)
    }
    fn read_utf8(&mut self, len: usize) -> AmfResult<String> {
        let buf = self.read_bytes(len)?;
        let str = String::from_utf8(buf)?;
        Ok(str)
    }
    fn read_and_record_utf8(&mut self) -> AmfResult<String> {
        match self.read_size_or_index()? {
            SizeOrIndex::Index(index) => {
                let result = self
                    .referenceable
                    .strings
                    .get(index)
                    .ok_or(AmfError::OutOfRangeReference { index })?;
                Ok(result.clone())
            }
            SizeOrIndex::Size(size) => {
                let str = self.read_utf8(size)?;
                if !str.is_empty() {
                    self.referenceable.strings.push(str.clone());
                }
                Ok(str)
            }
        }
    }
    fn read_and_record_object<F>(&mut self, f: F) -> AmfResult<Value>
    where
        F: FnOnce(&mut Self, usize) -> AmfResult<Value>,
    {
        match self.read_size_or_index()? {
            SizeOrIndex::Index(index) => self
                .referenceable
                .objects
                .get(index)
                .ok_or(AmfError::OutOfRangeReference { index })
                .and_then(|v| {
                    if *v == Value::Null {
                        Err(AmfError::CircularReference { index })
                    } else {
                        Ok(v.clone())
                    }
                }),
            SizeOrIndex::Size(size) => {
                let index = self.referenceable.objects.len();
                self.referenceable.objects.push(Value::Null);
                let result = f(self, size)?;
                self.referenceable.objects[index] = result.clone();
                Ok(result)
            }
        }
    }

    fn read_trait(&mut self, size: usize) -> AmfResult<Amf3Trait> {
        if (size & 0b1) == 0 {
            let index = size >> 1;
            let result = self
                .referenceable
                .traits
                .get(index)
                .ok_or(AmfError::OutOfRangeReference { index })?;
            return Ok(result.clone());
        }

        if (size & 0b10) != 0 {
            let class_name = self.read_and_record_utf8()?;
            return Err(AmfError::UnsupportedExternalizable { name: class_name });
        }

        let is_dynamic = (size & 0b100) != 0;
        let field_num = size >> 3;
        let class_name = self.read_and_record_utf8()?;
        let fields = (0..field_num)
            .map(|_| self.read_and_record_utf8())
            .collect::<AmfResult<_>>()?;
        let result = Amf3Trait {
            class_name: if class_name.is_empty() {
                None
            } else {
                Some(class_name)
            },
            is_dynamic,
            fields,
        };

        Ok(result)
    }

    fn read_and_record_trait(&mut self, size: usize) -> AmfResult<Amf3Trait> {
        let result = self.read_trait(size)?;
        self.referenceable.traits.push(result.clone());
        Ok(result)
    }

    pub fn read_integer(&mut self) -> AmfResult<Value> {
        let result = self.read_u29()? as i32;
        let result = if result >= (1 << 28) {
            result - (1 << 29)
        } else {
            result
        };
        Ok(Value::Integer(result))
    }
    pub fn read_double(&mut self) -> AmfResult<Value> {
        let result = self.inner.read_f64::<BigEndian>()?;
        Ok(Value::Double(result))
    }
    pub fn read_string(&mut self) -> AmfResult<Value> {
        let str = self.read_and_record_utf8()?;
        Ok(Value::String(str))
    }
    pub fn read_xml_document(&mut self) -> AmfResult<Value> {
        self.read_and_record_object(|this, size| this.read_utf8(size).map(Value::XMLDocument))
    }
    pub fn read_date(&mut self) -> AmfResult<Value> {
        self.read_and_record_object(|this, _| {
            let millis_timestamp = this.inner.read_f64::<BigEndian>()?;
            if !(millis_timestamp.is_finite() && millis_timestamp.is_sign_positive()) {
                Err(AmfError::InvalidDate {
                    milliseconds: millis_timestamp,
                })
            } else {
                Ok(Value::Date {
                    millis_timestamp: time::Duration::from_millis(millis_timestamp as u64),
                })
            }
        })
    }
    fn read_pairs(&mut self) -> AmfResult<Vec<(String, Value)>> {
        let mut result = Vec::new();
        loop {
            let key = self.read_and_record_utf8()?;
            if key.is_empty() {
                return Ok(result);
            }
            let value = self.read()?;
            result.push((key, value));
        }
    }
    pub fn read_array(&mut self) -> AmfResult<Value> {
        self.read_and_record_object(|this, size| {
            let assoc_entries = this.read_pairs()?;
            let dense_entries = (0..size).map(|_| this.read()).collect::<AmfResult<_>>()?;
            Ok(Value::Array {
                assoc_entries,
                dense_entries,
            })
        })
    }
    pub fn read_object(&mut self) -> AmfResult<Value> {
        self.read_and_record_object(|this, size| {
            let amf3_trait = this.read_and_record_trait(size)?;
            let mut entries = amf3_trait
                .fields
                .iter()
                .map(|key| {
                    let value = this.read()?;
                    Ok((key.clone(), value))
                })
                .collect::<AmfResult<Vec<_>>>()?;
            if amf3_trait.is_dynamic {
                entries.extend(this.read_pairs()?);
            }
            Ok(Value::Object {
                name: amf3_trait.class_name,
                sealed_fields_count: amf3_trait.fields.len(),
                entries,
            })
        })
    }
    pub fn read_xml(&mut self) -> AmfResult<Value> {
        self.read_and_record_object(|this, len| this.read_utf8(len).map(Value::XML))
    }
    pub fn read_byte_array(&mut self) -> AmfResult<Value> {
        self.read_and_record_object(|this, len| this.read_bytes(len).map(Value::ByteArray))
    }
    pub fn read_i32_vector(&mut self) -> AmfResult<Value> {
        self.read_and_record_object(|this, count| {
            let is_fixed = this.inner.read_u8()? != 0;
            let entries = (0..count)
                .map(|_| this.inner.read_i32::<BigEndian>())
                .collect::<Result<_, _>>()?;
            Ok(Value::I32Vector { is_fixed, entries })
        })
    }
    pub fn read_u32_vector(&mut self) -> AmfResult<Value> {
        self.read_and_record_object(|this, count| {
            let is_fixed = this.inner.read_u8()? != 0;
            let entries = (0..count)
                .map(|_| this.inner.read_u32::<BigEndian>())
                .collect::<Result<_, _>>()?;
            Ok(Value::U32Vector { is_fixed, entries })
        })
    }
    pub fn read_double_vector(&mut self) -> AmfResult<Value> {
        self.read_and_record_object(|this, count| {
            let is_fixed = this.inner.read_u8()? != 0;
            let entries = (0..count)
                .map(|_| this.inner.read_f64::<BigEndian>())
                .collect::<Result<_, _>>()?;
            Ok(Value::DoubleVector { is_fixed, entries })
        })
    }
    pub fn read_object_vector(&mut self) -> AmfResult<Value> {
        self.read_and_record_object(|this, count| {
            let is_fixed = this.inner.read_u8()? != 0;
            let class_name = this.read_and_record_utf8()?;
            let entries = (0..count).map(|_| this.read()).collect::<AmfResult<_>>()?;
            Ok(Value::ObjectVector {
                is_fixed,
                entries,
                class_name: if class_name.is_empty() {
                    None
                } else {
                    Some(class_name)
                },
            })
        })
    }
    pub fn read_dictionary(&mut self) -> AmfResult<Value> {
        self.read_and_record_object(|this, count| {
            let is_weak = this.inner.read_u8()? == 1;
            let entries = (0..count)
                .map(|_| {
                    let key = this.read()?;
                    let value = this.read()?;
                    Ok((key, value))
                })
                .collect::<AmfResult<_>>()?;
            Ok(Value::Dictionary { is_weak, entries })
        })
    }
}

impl<R: io::Read> ReadFrom<R> for Value {
    type Error = AmfError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        Reader::new(reader).read()
    }
}
