use core::time;
use std::io;

use crate::errors::{AmfError, AmfResult};
use byteorder::{BigEndian, WriteBytesExt};
use utils::traits::writer::WriteTo;

use super::{
    Value,
    amf3_marker::{self},
};

impl<W: io::Write> WriteTo<W> for Value {
    type Error = AmfError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        match self {
            Value::Undefined => Self::write_undefined(writer),
            Value::Null => Self::write_null(writer),
            Value::Boolean(value) => {
                if *value {
                    Self::write_true(writer)
                } else {
                    Self::write_false(writer)
                }
            }
            Value::Integer(value) => Self::write_integer(writer, *value),
            Value::Double(value) => Self::write_double(writer, *value),
            Value::String(value) => Self::write_string(writer, value),
            Value::XMLDocument(value) => Self::write_xml_document(writer, value),
            Value::Date { millis_timestamp } => Self::write_date(writer, *millis_timestamp),
            Value::Array {
                assoc_entries,
                dense_entries,
            } => Self::write_array(writer, assoc_entries, dense_entries),
            Value::Object {
                name,
                sealed_fields_count,
                entries,
            } => Self::write_object(writer, name, *sealed_fields_count, entries),
            Value::XML(value) => Self::write_xml(writer, value),
            Value::ByteArray(value) => Self::write_byte_array(writer, value),
            Value::I32Vector { is_fixed, entries } => {
                Self::write_i32_vector(writer, *is_fixed, entries)
            }
            Value::U32Vector { is_fixed, entries } => {
                Self::write_u32_vector(writer, *is_fixed, entries)
            }
            Value::DoubleVector { is_fixed, entries } => {
                Self::write_double_vector(writer, *is_fixed, entries)
            }
            Value::ObjectVector {
                is_fixed,
                class_name,
                entries,
            } => Self::write_object_vector(writer, class_name, *is_fixed, entries),
            Value::Dictionary { is_weak, entries } => {
                Self::write_dictionary(writer, *is_weak, entries)
            }
        }
    }
}

impl Value {
    pub fn write_undefined<W: io::Write>(writer: &mut W) -> AmfResult<()> {
        writer.write_u8(amf3_marker::UNDEFINED)?;
        Ok(())
    }

    pub fn write_null<W: io::Write>(writer: &mut W) -> AmfResult<()> {
        writer.write_u8(amf3_marker::NULL)?;
        Ok(())
    }

    pub fn write_false<W: io::Write>(writer: &mut W) -> AmfResult<()> {
        writer.write_u8(amf3_marker::FALSE)?;
        Ok(())
    }

    pub fn write_true<W: io::Write>(writer: &mut W) -> AmfResult<()> {
        writer.write_u8(amf3_marker::TRUE)?;
        Ok(())
    }

    fn write_u29_inner<W: io::Write>(writer: &mut W, u29: u32) -> AmfResult<()> {
        match u29 {
            i if i < 0x80 => {
                writer.write_u8(i as u8)?;
            }
            i if i < 0x4000 => {
                writer.write_u8(((u29 >> 7) | 0b1000_0000) as u8)?;
                writer.write_u8((u29 & 0b0111_1111) as u8)?;
            }
            i if i < 0x20_0000 => {
                writer.write_u8(((u29 >> 14) | 0b1000_0000) as u8)?;
                writer.write_u8(((u29 >> 7) | 0b1000_0000) as u8)?;
                writer.write_u8((u29 & 0b0111_1111) as u8)?;
            }
            i if i < 0x4000_0000 => {
                writer.write_u8(((u29 >> 22) | 0b1000_0000) as u8)?;
                writer.write_u8(((u29 >> 15) | 0b1000_0000) as u8)?;
                writer.write_u8(((u29 >> 8) | 0b1000_0000) as u8)?;
                writer.write_u8((u29 & 0b1111_1111) as u8)?;
            }
            _ => return Err(AmfError::U29OutOfRange { value: u29 }),
        }
        Ok(())
    }

    pub fn write_integer<W: io::Write>(writer: &mut W, value: i32) -> AmfResult<()> {
        writer.write_u8(amf3_marker::INTEGER)?;
        let u29 = if value >= 0 {
            value as u32
        } else {
            ((1 << 29) + value) as u32
        };
        Self::write_u29_inner(writer, u29)?;
        Ok(())
    }

    pub fn write_double<W: io::Write>(writer: &mut W, value: f64) -> AmfResult<()> {
        writer.write_u8(amf3_marker::DOUBLE)?;
        writer.write_f64::<BigEndian>(value)?;
        Ok(())
    }

    fn write_size_inner<W: io::Write>(writer: &mut W, size: usize) -> AmfResult<()> {
        if size >= (1 << 28) {
            return Err(AmfError::SizeOutOfRange { value: size });
        }
        let not_reference_bit = 1;
        Self::write_u29_inner(writer, ((size << 1) | not_reference_bit) as u32)?;
        Ok(())
    }
    fn write_utf8_inner<W: io::Write>(writer: &mut W, value: &str) -> AmfResult<()> {
        Self::write_size_inner(writer, value.len())?;
        writer.write_all(value.as_bytes())?;
        Ok(())
    }
    pub fn write_string<W: io::Write>(writer: &mut W, value: &str) -> AmfResult<()> {
        writer.write_u8(amf3_marker::STRING)?;
        Self::write_utf8_inner(writer, value)?;
        Ok(())
    }
    pub fn write_xml_document<W: io::Write>(writer: &mut W, xml_doc: &str) -> AmfResult<()> {
        writer.write_u8(amf3_marker::XML_DOCUMENT)?;
        Self::write_utf8_inner(writer, xml_doc)?;
        Ok(())
    }

    pub fn write_date<W: io::Write>(
        writer: &mut W,
        millis_timestamp: time::Duration,
    ) -> AmfResult<()> {
        writer.write_u8(amf3_marker::DATE)?;
        Self::write_size_inner(writer, 0)?;
        writer.write_f64::<BigEndian>(millis_timestamp.as_millis() as f64)?;
        Ok(())
    }

    fn write_pairs_inner<W: io::Write>(writer: &mut W, pairs: &[(String, Value)]) -> AmfResult<()> {
        for (key, value) in pairs {
            Self::write_utf8_inner(writer, key)?;
            value.write_to(writer)?;
        }
        Self::write_utf8_inner(writer, "")?;
        Ok(())
    }
    pub fn write_array<W: io::Write>(
        writer: &mut W,
        assoc: &[(String, Value)],
        dense: &[Value],
    ) -> AmfResult<()> {
        writer.write_u8(amf3_marker::ARRAY)?;
        Self::write_size_inner(writer, dense.len())?;
        Self::write_pairs_inner(writer, assoc)?;
        dense
            .iter()
            .try_for_each(|value| value.write_to(writer))?;
        Ok(())
    }

    fn write_trait_inner<W: io::Write>(
        writer: &mut W,
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
        Self::write_size_inner(writer, u28)?;
        let class_name = class_name.as_ref().map_or("", |s| s);
        Self::write_utf8_inner(writer, class_name)?;
        for (key, _) in entries.iter().take(sealed_count) {
            Self::write_utf8_inner(writer, key)?;
        }
        Ok(())
    }

    pub fn write_object<W: io::Write>(
        writer: &mut W,
        class_name: &Option<String>,
        sealed_count: usize,
        entries: &[(String, Value)],
    ) -> AmfResult<()> {
        writer.write_u8(amf3_marker::OBJECT)?;
        Self::write_trait_inner(writer, class_name, entries, sealed_count)?;
        entries
            .iter()
            .take(sealed_count)
            .try_for_each(|(_, value)| value.write_to(writer))?;

        if entries.len() > sealed_count {
            Self::write_pairs_inner(writer, &entries[sealed_count..])?;
        }
        Ok(())
    }

    pub fn write_xml<W: io::Write>(writer: &mut W, xml: &str) -> AmfResult<()> {
        writer.write_u8(amf3_marker::XML)?;
        Self::write_utf8_inner(writer, xml)?;
        Ok(())
    }

    pub fn write_byte_array<W: io::Write>(writer: &mut W, bytes: &[u8]) -> AmfResult<()> {
        writer.write_u8(amf3_marker::BYTE_ARRAY)?;
        Self::write_size_inner(writer, bytes.len())?;
        writer.write_all(bytes)?;
        Ok(())
    }

    pub fn write_i32_vector<W: io::Write>(
        writer: &mut W,
        is_fixed: bool,
        value: &[i32],
    ) -> AmfResult<()> {
        writer.write_u8(amf3_marker::VECTOR_INT)?;
        Self::write_size_inner(writer, value.len())?;
        writer.write_u8(is_fixed as u8)?;
        value
            .iter()
            .try_for_each(|v| writer.write_i32::<BigEndian>(*v))?;

        Ok(())
    }

    pub fn write_u32_vector<W: io::Write>(
        writer: &mut W,
        is_fixed: bool,
        value: &[u32],
    ) -> AmfResult<()> {
        writer.write_u8(amf3_marker::VECTOR_UINT)?;
        Self::write_size_inner(writer, value.len())?;
        writer.write_u8(is_fixed as u8)?;
        value
            .iter()
            .try_for_each(|v| writer.write_u32::<BigEndian>(*v))?;

        Ok(())
    }

    pub fn write_double_vector<W: io::Write>(
        writer: &mut W,
        is_fixed: bool,
        value: &[f64],
    ) -> AmfResult<()> {
        writer.write_u8(amf3_marker::VECTOR_DOUBLE)?;
        Self::write_size_inner(writer, value.len())?;
        writer.write_u8(is_fixed as u8)?;
        value
            .iter()
            .try_for_each(|v| writer.write_f64::<BigEndian>(*v))?;

        Ok(())
    }

    pub fn write_object_vector<W: io::Write>(
        writer: &mut W,
        class_name: &Option<String>,
        is_fixed: bool,
        value: &[Value],
    ) -> AmfResult<()> {
        writer.write_u8(amf3_marker::VECTOR_OBJECT)?;
        Self::write_size_inner(writer, value.len())?;
        writer.write_u8(is_fixed as u8)?;
        Self::write_utf8_inner(writer, class_name.as_ref().map_or("", |s| s))?;
        value
            .iter()
            .try_for_each(|value| value.write_to(writer))?;

        Ok(())
    }

    pub fn write_dictionary<W: io::Write>(
        writer: &mut W,
        is_weak: bool,
        entries: &[(Value, Value)],
    ) -> AmfResult<()> {
        writer.write_u8(amf3_marker::DICTIONARY)?;
        Self::write_size_inner(writer, entries.len())?;
        writer.write_u8(is_weak as u8)?;
        entries.iter().try_for_each(|(key, value)| {
            key.write_to(writer)
                .and_then(|_| value.write_to(writer))
        })?;

        Ok(())
    }
}
