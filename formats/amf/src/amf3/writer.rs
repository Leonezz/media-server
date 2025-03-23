use core::time;
use std::io;

use crate::errors::{AmfError, AmfResult};
use byteorder::{BigEndian, WriteBytesExt};

use super::{
    Value,
    amf3_marker::{self},
};

pub struct Writer<W> {
    inner: W,
}

impl<W> Writer<W> {
    pub fn into_inner(self) -> W {
        self.inner
    }
    pub fn inner(&self) -> &W {
        &self.inner
    }
    pub fn inner_mut(&mut self) -> &mut W {
        &mut self.inner
    }
}

impl<W> Writer<W>
where
    W: io::Write,
{
    pub fn new(inner: W) -> Self {
        Self { inner }
    }
    pub fn write(&mut self, value: &Value) -> AmfResult<()> {
        match *value {
            Value::Undefined => self.write_undefined(),
            Value::Null => self.write_null(),
            Value::Boolean(value) => {
                if value {
                    self.write_true()
                } else {
                    self.write_false()
                }
            }
            Value::Integer(value) => self.write_integer(value),
            Value::Double(value) => self.write_double(value),
            Value::String(ref value) => self.write_string(value),
            Value::XMLDocument(ref value) => self.write_xml_document(value),
            Value::Date { millis_timestamp } => self.write_date(millis_timestamp),
            Value::Array {
                ref assoc_entries,
                ref dense_entries,
            } => self.write_array(assoc_entries, dense_entries),
            Value::Object {
                ref name,
                sealed_fields_count,
                ref entries,
            } => self.write_object(name, sealed_fields_count, entries),
            Value::XML(ref value) => self.write_xml(value),
            Value::ByteArray(ref value) => self.write_byte_array(value),
            Value::I32Vector {
                is_fixed,
                ref entries,
            } => self.write_i32_vector(is_fixed, entries),
            Value::U32Vector {
                is_fixed,
                ref entries,
            } => self.write_u32_vector(is_fixed, entries),
            Value::DoubleVector {
                is_fixed,
                ref entries,
            } => self.write_double_vector(is_fixed, entries),
            Value::ObjectVector {
                is_fixed,
                ref class_name,
                ref entries,
            } => self.write_object_vector(class_name, is_fixed, entries),
            Value::Dictionary {
                is_weak,
                ref entries,
            } => self.write_dictionary(is_weak, entries),
        }
    }

    fn write_undefined(&mut self) -> AmfResult<()> {
        self.inner.write_u8(amf3_marker::UNDEFINED)?;
        Ok(())
    }

    fn write_null(&mut self) -> AmfResult<()> {
        self.inner.write_u8(amf3_marker::NULL)?;
        Ok(())
    }

    fn write_false(&mut self) -> AmfResult<()> {
        self.inner.write_u8(amf3_marker::FALSE)?;
        Ok(())
    }

    fn write_true(&mut self) -> AmfResult<()> {
        self.inner.write_u8(amf3_marker::TRUE)?;
        Ok(())
    }

    fn write_u29_inner(&mut self, u29: u32) -> AmfResult<()> {
        match u29 {
            i if i < 0x80 => {
                self.inner.write_u8(i as u8)?;
            }
            i if i < 0x4000 => {
                self.inner.write_u8(((u29 >> 7) | 0b1000_0000) as u8)?;
                self.inner.write_u8((u29 & 0b0111_1111) as u8)?;
            }
            i if i < 0x20_0000 => {
                self.inner.write_u8(((u29 >> 14) | 0b1000_0000) as u8)?;
                self.inner.write_u8(((u29 >> 7) | 0b1000_0000) as u8)?;
                self.inner.write_u8((u29 & 0b0111_1111) as u8)?;
            }
            i if i < 0x4000_0000 => {
                self.inner.write_u8(((u29 >> 22) | 0b1000_0000) as u8)?;
                self.inner.write_u8(((u29 >> 15) | 0b1000_0000) as u8)?;
                self.inner.write_u8(((u29 >> 8) | 0b1000_0000) as u8)?;
                self.inner.write_u8((u29 & 0b1111_1111) as u8)?;
            }
            _ => return Err(AmfError::U29OutOfRange { value: u29 }),
        }
        Ok(())
    }

    fn write_integer(&mut self, value: i32) -> AmfResult<()> {
        self.inner.write_u8(amf3_marker::INTEGER)?;
        let u29 = if value >= 0 {
            value as u32
        } else {
            ((1 << 29) + value) as u32
        };
        self.write_u29_inner(u29)?;
        Ok(())
    }

    fn write_double(&mut self, value: f64) -> AmfResult<()> {
        self.inner.write_u8(amf3_marker::DOUBLE)?;
        self.inner.write_f64::<BigEndian>(value)?;
        Ok(())
    }

    fn write_size_inner(&mut self, size: usize) -> AmfResult<()> {
        if size >= (1 << 28) {
            return Err(AmfError::SizeOutOfRange { value: size });
        }
        let not_reference_bit = 1;
        self.write_u29_inner(((size << 1) | not_reference_bit) as u32)?;
        Ok(())
    }
    fn write_utf8_inner(&mut self, value: &str) -> AmfResult<()> {
        self.write_size_inner(value.len())?;
        self.inner.write_all(value.as_bytes())?;
        Ok(())
    }
    fn write_string(&mut self, value: &str) -> AmfResult<()> {
        self.inner.write_u8(amf3_marker::STRING)?;
        self.write_utf8_inner(value)?;
        Ok(())
    }
    fn write_xml_document(&mut self, xml_doc: &str) -> AmfResult<()> {
        self.inner.write_u8(amf3_marker::XML_DOCUMENT)?;
        self.write_utf8_inner(xml_doc)?;
        Ok(())
    }

    fn write_date(&mut self, millis_timestamp: time::Duration) -> AmfResult<()> {
        self.inner.write_u8(amf3_marker::DATE)?;
        self.write_size_inner(0)?;
        self.inner
            .write_f64::<BigEndian>(millis_timestamp.as_millis() as f64)?;
        Ok(())
    }

    fn write_pairs_inner(&mut self, pairs: &[(String, Value)]) -> AmfResult<()> {
        for (key, value) in pairs {
            self.write_utf8_inner(key)?;
            self.write(value)?;
        }
        self.write_utf8_inner("")?;
        Ok(())
    }
    fn write_array(&mut self, assoc: &[(String, Value)], dense: &[Value]) -> AmfResult<()> {
        self.inner.write_u8(amf3_marker::ARRAY)?;
        self.write_size_inner(dense.len())?;
        self.write_pairs_inner(assoc)?;
        dense.iter().try_for_each(|value| self.write(value))?;
        Ok(())
    }

    fn write_trait_inner(
        &mut self,
        class_name: &Option<String>,
        entries: &[(String, Value)],
        sealed_count: usize,
    ) -> AmfResult<()> {
        if sealed_count > entries.len() {
            return Err(AmfError::Amf3TraitInvalid {
                entries: Vec::from(entries),
                sealed_count,
            });
        }
        let not_reference_bit = 1_usize;
        let is_externalizable = false as usize;
        let is_dynamic = (sealed_count < entries.len()) as usize;
        let u28 =
            (sealed_count << 3) | (is_dynamic << 2) | (is_externalizable << 1) | not_reference_bit;
        self.write_size_inner(u28)?;
        let class_name = class_name.as_ref().map_or("", |s| s);
        self.write_utf8_inner(class_name)?;
        for (key, _) in entries.iter().take(sealed_count) {
            self.write_utf8_inner(key)?;
        }
        Ok(())
    }

    fn write_object(
        &mut self,
        class_name: &Option<String>,
        sealed_count: usize,
        entries: &[(String, Value)],
    ) -> AmfResult<()> {
        self.inner.write_u8(amf3_marker::OBJECT)?;
        self.write_trait_inner(class_name, entries, sealed_count)?;
        for (_, value) in entries.iter().take(sealed_count) {
            self.write(value)?;
        }
        if entries.len() > sealed_count {
            self.write_pairs_inner(&entries[sealed_count..])?;
        }
        Ok(())
    }

    fn write_xml(&mut self, xml: &str) -> AmfResult<()> {
        self.inner.write_u8(amf3_marker::XML)?;
        self.write_utf8_inner(xml)?;
        Ok(())
    }

    fn write_byte_array(&mut self, bytes: &[u8]) -> AmfResult<()> {
        self.inner.write_u8(amf3_marker::BYTE_ARRAY)?;
        self.write_size_inner(bytes.len())?;
        self.inner.write_all(bytes)?;
        Ok(())
    }

    fn write_i32_vector(&mut self, is_fixed: bool, value: &[i32]) -> AmfResult<()> {
        self.inner.write_u8(amf3_marker::VECTOR_INT)?;
        self.write_size_inner(value.len())?;
        self.inner.write_u8(is_fixed as u8)?;
        for &v in value {
            self.inner.write_i32::<BigEndian>(v)?;
        }
        Ok(())
    }

    fn write_u32_vector(&mut self, is_fixed: bool, value: &[u32]) -> AmfResult<()> {
        self.inner.write_u8(amf3_marker::VECTOR_UINT)?;
        self.write_size_inner(value.len())?;
        self.inner.write_u8(is_fixed as u8)?;
        for &v in value {
            self.inner.write_u32::<BigEndian>(v)?;
        }
        Ok(())
    }

    fn write_double_vector(&mut self, is_fixed: bool, value: &[f64]) -> AmfResult<()> {
        self.inner.write_u8(amf3_marker::VECTOR_DOUBLE)?;
        self.write_size_inner(value.len())?;
        self.inner.write_u8(is_fixed as u8)?;
        for &v in value {
            self.inner.write_f64::<BigEndian>(v)?;
        }
        Ok(())
    }

    fn write_object_vector(
        &mut self,
        class_name: &Option<String>,
        is_fixed: bool,
        value: &[Value],
    ) -> AmfResult<()> {
        self.inner.write_u8(amf3_marker::VECTOR_OBJECT)?;
        self.write_size_inner(value.len())?;
        self.inner.write_u8(is_fixed as u8)?;
        self.write_utf8_inner(class_name.as_ref().map_or("", |s| s))?;
        for v in value {
            self.write(v)?;
        }
        Ok(())
    }

    fn write_dictionary(&mut self, is_weak: bool, entries: &[(Value, Value)]) -> AmfResult<()> {
        self.inner.write_u8(amf3_marker::DICTIONARY)?;
        self.write_size_inner(entries.len())?;
        self.inner.write_u8(is_weak as u8)?;
        for (key, value) in entries {
            self.write(key)?;
            self.write(value)?;
        }
        Ok(())
    }
}

