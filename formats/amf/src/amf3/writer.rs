use core::time;
use std::io;

use crate::errors::{AmfWriteError, AmfWriteResult};
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
    pub fn write(&mut self, value: &Value) -> AmfWriteResult {
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

    fn write_undefined(&mut self) -> AmfWriteResult {
        self.inner.write_u8(amf3_marker::UNDEFINED)?;
        Ok(())
    }

    fn write_null(&mut self) -> AmfWriteResult {
        self.inner.write_u8(amf3_marker::NULL)?;
        Ok(())
    }

    fn write_false(&mut self) -> AmfWriteResult {
        self.inner.write_u8(amf3_marker::FALSE)?;
        Ok(())
    }

    fn write_true(&mut self) -> AmfWriteResult {
        self.inner.write_u8(amf3_marker::TRUE)?;
        Ok(())
    }

    fn write_u29_inner(&mut self, u29: u32) -> AmfWriteResult {
        match u29 {
            i if i < 0x80 => {
                self.inner.write_u8(i as u8)?;
            }
            i if i < 0x4000 => {
                self.inner.write_u8(((u29 >> 7) | 0b1000_0000) as u8)?;
                self.inner.write_u8(((u29 >> 0) & 0b0111_1111) as u8)?;
            }
            i if i < 0x20_0000 => {
                self.inner.write_u8(((u29 >> 14) | 0b1000_0000) as u8)?;
                self.inner.write_u8(((u29 >> 7) | 0b1000_0000) as u8)?;
                self.inner.write_u8(((u29 >> 0) & 0b0111_1111) as u8)?;
            }
            i if i < 0x4000_0000 => {
                self.inner.write_u8(((u29 >> 22) | 0b1000_0000) as u8)?;
                self.inner.write_u8(((u29 >> 15) | 0b1000_0000) as u8)?;
                self.inner.write_u8(((u29 >> 8) | 0b1000_0000) as u8)?;
                self.inner.write_u8(((u29 >> 0) & 0b1111_1111) as u8)?;
            }
            _ => return Err(AmfWriteError::U29OutOfRange { value: u29 }),
        }
        Ok(())
    }

    fn write_integer(&mut self, value: i32) -> AmfWriteResult {
        self.inner.write_u8(amf3_marker::INTEGER)?;
        let u29 = if value >= 0 {
            value as u32
        } else {
            ((1 << 29) + value) as u32
        };
        self.write_u29_inner(u29)?;
        Ok(())
    }

    fn write_double(&mut self, value: f64) -> AmfWriteResult {
        self.inner.write_u8(amf3_marker::DOUBLE)?;
        self.inner.write_f64::<BigEndian>(value)?;
        Ok(())
    }

    fn write_size_inner(&mut self, size: usize) -> AmfWriteResult {
        if size >= (1 << 28) {
            return Err(AmfWriteError::SizeOutOfRange { value: size });
        }
        let not_reference_bit = 1;
        self.write_u29_inner(((size << 1) | not_reference_bit) as u32)?;
        Ok(())
    }
    fn write_utf8_inner(&mut self, value: &str) -> AmfWriteResult {
        self.write_size_inner(value.len())?;
        self.inner.write_all(value.as_bytes())?;
        Ok(())
    }
    fn write_string(&mut self, value: &str) -> AmfWriteResult {
        self.inner.write_u8(amf3_marker::STRING)?;
        self.write_utf8_inner(value)?;
        Ok(())
    }
    fn write_xml_document(&mut self, xml_doc: &str) -> AmfWriteResult {
        self.inner.write_u8(amf3_marker::XML_DOCUMENT)?;
        self.write_utf8_inner(xml_doc)?;
        Ok(())
    }

    fn write_date(&mut self, millis_timestamp: time::Duration) -> AmfWriteResult {
        self.inner.write_u8(amf3_marker::DATE)?;
        self.write_size_inner(0)?;
        self.inner
            .write_f64::<BigEndian>(millis_timestamp.as_millis() as f64)?;
        Ok(())
    }

    fn write_pairs_inner(&mut self, pairs: &[(String, Value)]) -> AmfWriteResult {
        for (key, value) in pairs {
            self.write_utf8_inner(&key)?;
            self.write(value)?;
        }
        self.write_utf8_inner("")?;
        Ok(())
    }
    fn write_array(&mut self, assoc: &[(String, Value)], dense: &[Value]) -> AmfWriteResult {
        self.inner.write_u8(amf3_marker::ARRAY)?;
        self.write_size_inner(dense.len())?;
        self.write_pairs_inner(assoc)?;
        dense
            .iter()
            .map(|value| self.write(value))
            .collect::<AmfWriteResult>()?;
        Ok(())
    }

    fn write_trait_inner(
        &mut self,
        class_name: &Option<String>,
        entries: &[(String, Value)],
        sealed_count: usize,
    ) -> AmfWriteResult {
        if sealed_count > entries.len() {
            return Err(AmfWriteError::Amf3TraitInvalid {
                entries: Vec::from(entries),
                sealed_count: sealed_count,
            });
        }
        let not_reference_bit = 1 as usize;
        let is_externalizable = false as usize;
        let is_dynamic = (sealed_count < entries.len()) as usize;
        let u28 =
            (sealed_count << 3) | (is_dynamic << 2) | (is_externalizable << 1) | not_reference_bit;
        self.write_size_inner(u28)?;
        let class_name = class_name.as_ref().map_or("", |s| s);
        self.write_utf8_inner(&class_name)?;
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
    ) -> AmfWriteResult {
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

    fn write_xml(&mut self, xml: &str) -> AmfWriteResult {
        self.inner.write_u8(amf3_marker::XML)?;
        self.write_utf8_inner(xml)?;
        Ok(())
    }

    fn write_byte_array(&mut self, bytes: &[u8]) -> AmfWriteResult {
        self.inner.write_u8(amf3_marker::BYTE_ARRAY)?;
        self.write_size_inner(bytes.len())?;
        self.inner.write_all(bytes)?;
        Ok(())
    }

    fn write_i32_vector(&mut self, is_fixed: bool, value: &[i32]) -> AmfWriteResult {
        self.inner.write_u8(amf3_marker::VECTOR_INT)?;
        self.write_size_inner(value.len())?;
        self.inner.write_u8(is_fixed as u8)?;
        for &v in value {
            self.inner.write_i32::<BigEndian>(v)?;
        }
        Ok(())
    }

    fn write_u32_vector(&mut self, is_fixed: bool, value: &[u32]) -> AmfWriteResult {
        self.inner.write_u8(amf3_marker::VECTOR_UINT)?;
        self.write_size_inner(value.len())?;
        self.inner.write_u8(is_fixed as u8)?;
        for &v in value {
            self.inner.write_u32::<BigEndian>(v)?;
        }
        Ok(())
    }

    fn write_double_vector(&mut self, is_fixed: bool, value: &[f64]) -> AmfWriteResult {
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
    ) -> AmfWriteResult {
        self.inner.write_u8(amf3_marker::VECTOR_OBJECT)?;
        self.write_size_inner(value.len())?;
        self.inner.write_u8(is_fixed as u8)?;
        self.write_utf8_inner(class_name.as_ref().map_or("", |s| s))?;
        for v in value {
            self.write(v)?;
        }
        Ok(())
    }

    fn write_dictionary(&mut self, is_weak: bool, entries: &[(Value, Value)]) -> AmfWriteResult {
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

#[cfg(test)]
mod tests {
    use core::time;

    use crate::amf3::{self, Value};

    use super::Writer;
    macro_rules! encode {
        ($value:expr) => {{
            let mut buf = Vec::new();
            let res = Writer::new(&mut buf).write(&$value);
            assert!(res.is_ok());
            buf
        }};
    }

    #[test]
    fn undefined() {
        assert_eq!(
            encode!(Value::Undefined),
            include_bytes!("../../test_data/amf3-undefined.bin")
        );
    }

    #[test]
    fn null() {
        assert_eq!(
            encode!(Value::Null),
            include_bytes!("../../test_data/amf3-null.bin")
        );
    }

    #[test]
    fn boolean() {
        assert_eq!(
            encode!(Value::Boolean(false)),
            include_bytes!("../../test_data/amf3-false.bin")
        );
        assert_eq!(
            encode!(Value::Boolean(true)),
            include_bytes!("../../test_data/amf3-true.bin")
        );
    }

    #[test]
    fn integer() {
        assert_eq!(
            encode!(Value::Integer(0)),
            include_bytes!("../../test_data/amf3-0.bin")
        );

        assert_eq!(
            encode!(Value::Integer(0b1000_0000)),
            include_bytes!("../../test_data/amf3-integer-2byte.bin")
        );

        assert_eq!(
            encode!(Value::Integer(0b100_0000_0000_0000)),
            include_bytes!("../../test_data/amf3-integer-3byte.bin")
        );

        assert_eq!(
            encode!(Value::Integer(-0x1000_0000)),
            include_bytes!("../../test_data/amf3-min.bin")
        );

        assert_eq!(
            encode!(Value::Integer(0xFFF_FFFF)),
            include_bytes!("../../test_data/amf3-max.bin")
        );
    }

    #[test]
    fn double() {
        assert_eq!(
            encode!(Value::Double(3.5)),
            include_bytes!("../../test_data/amf3-float.bin")
        );

        assert_eq!(
            encode!(Value::Double(2f64.powf(1000f64))),
            include_bytes!("../../test_data/amf3-bignum.bin")
        );

        assert_eq!(
            encode!(Value::Double(-0x1000_0001 as f64)),
            include_bytes!("../../test_data/amf3-large-min.bin")
        );

        assert_eq!(
            encode!(Value::Double(268_435_456_f64)),
            include_bytes!("../../test_data/amf3-large-max.bin")
        )
    }

    #[test]
    fn string() {
        assert_eq!(
            encode!(Value::String("String . String".to_string())),
            include_bytes!("../../test_data/amf3-string.bin")
        );

        assert_eq!(
            encode!(Value::Array {
                assoc_entries: vec![],
                dense_entries: vec![
                    Value::Integer(5),
                    Value::String("Shift テスト".to_string()),
                    Value::String("UTF テスト".to_string()),
                    Value::Integer(5)
                ]
            }),
            include_bytes!("../../test_data/amf3-complex-encoded-string-array.bin")
        );
    }

    #[test]
    fn xml_document() {
        assert_eq!(
            encode!(Value::XMLDocument(
                "<parent><child prop=\"test\" /></parent>".to_string()
            )),
            include_bytes!("../../test_data/amf3-xml-doc.bin")
        );
    }

    #[test]
    fn date() {
        assert_eq!(
            encode!(Value::Date {
                millis_timestamp: time::Duration::from_secs(0)
            }),
            include_bytes!("../../test_data/amf3-date.bin")
        );
    }

    #[test]
    fn array() {
        assert_eq!(
            encode!(Value::Array {
                assoc_entries: vec![],
                dense_entries: vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(3),
                    Value::Integer(4),
                    Value::Integer(5)
                ]
            }),
            include_bytes!("../../test_data/amf3-primitive-array.bin")
        );

        let value = Value::Array {
            assoc_entries: vec![
                ("2".to_string(), Value::String("bar3".to_string())),
                ("foo".to_string(), Value::String("bar".to_string())),
                ("asdf".to_string(), Value::String("fdsa".to_string())),
            ],
            dense_entries: vec![
                Value::String("bar".to_string()),
                Value::String("bar1".to_string()),
                Value::String("bar2".to_string()),
            ],
        };

        let mut buf: Vec<u8> = Vec::new();
        amf3::Writer::new(&mut buf).write(&value).unwrap();
        assert_eq!(amf3::Decoder::new(&mut &buf[..]).decode().unwrap(), value);
    }

    #[test]
    fn object() {
        assert_eq!(
            encode!(Value::Object {
                name: Some("org.amf.ASClass".to_string()),
                sealed_fields_count: 2,
                entries: vec![
                    ("foo".to_string(), Value::String("bar".to_string())),
                    ("baz".to_string(), Value::Null)
                ]
            }),
            include_bytes!("../../test_data/amf3-typed-object.bin")
        );

        assert_eq!(
            encode!(Value::Object {
                name: None,
                sealed_fields_count: 0,
                entries: vec![
                    ("foo".to_string(), Value::String("bar".to_string())),
                    ("answer".to_string(), Value::Integer(42))
                ]
            }),
            include_bytes!("../../test_data/amf3-hash.bin")
        );
    }

    #[test]
    fn xml() {
        assert_eq!(
            encode!(Value::XML(
                "<parent><child prop=\"test\"/></parent>".to_string()
            )),
            include_bytes!("../../test_data/amf3-xml.bin")
        );
    }

    #[test]
    fn byte_array() {
        assert_eq!(
            encode!(Value::ByteArray(vec![
                0, 3, 227, 129, 147, 227, 130, 140, 116, 101, 115, 116, 64
            ])),
            include_bytes!("../../test_data/amf3-byte-array.bin")
        );
    }

    #[test]
    fn i32_vector() {
        assert_eq!(
            encode!(Value::I32Vector {
                is_fixed: false,
                entries: vec![4, -20, 12],
            }),
            include_bytes!("../../test_data/amf3-vector-int.bin")
        );
    }

    #[test]
    fn u32_vector() {
        assert_eq!(
            encode!(Value::U32Vector {
                is_fixed: false,
                entries: vec![4, 20, 12]
            }),
            include_bytes!("../../test_data/amf3-vector-uint.bin")
        );
    }

    #[test]
    fn double_vector() {
        assert_eq!(
            encode!(Value::DoubleVector {
                is_fixed: false,
                entries: vec![4.3, -20.6]
            }),
            include_bytes!("../../test_data/amf3-vector-double.bin")
        );
    }

    #[test]
    fn object_vector() {
        let value = Value::ObjectVector {
            is_fixed: false,
            class_name: Some("org.amf.ASClass".to_string()),
            entries: vec![
                Value::Object {
                    name: Some("org.amf.ASClass".to_string()),
                    sealed_fields_count: 2,
                    entries: vec![
                        ("foo".to_string(), Value::String("foo".to_string())),
                        ("baz".to_string(), Value::Null),
                    ],
                },
                Value::Object {
                    name: Some("org.amf.ASClass".to_string()),
                    sealed_fields_count: 2,
                    entries: vec![
                        ("foo".to_string(), Value::String("bar".to_string())),
                        ("baz".to_string(), Value::Null),
                    ],
                },
                Value::Object {
                    name: Some("org.amf.ASClass".to_string()),
                    sealed_fields_count: 2,
                    entries: vec![
                        ("foo".to_string(), Value::String("baz".to_string())),
                        ("baz".to_string(), Value::Null),
                    ],
                },
            ],
        };
        let buf = encode!(value);

        assert_eq!(amf3::Decoder::new(&mut &buf[..]).decode().unwrap(), value);
    }

    #[test]
    fn dictionary() {
        let value = Value::Dictionary {
            is_weak: false,
            entries: vec![
                (
                    Value::String("bar".to_string()),
                    Value::String("asdf1".to_string()),
                ),
                (
                    Value::Object {
                        name: Some("org.amf.ASClass".to_string()),
                        sealed_fields_count: 2,
                        entries: vec![
                            ("foo".to_string(), Value::String("baz".to_string())),
                            ("baz".to_string(), Value::Null),
                        ],
                    },
                    Value::String("asdf2".to_string()),
                ),
            ],
        };
        let buf = encode!(value);
        assert_eq!(amf3::Decoder::new(&mut &buf[..]).decode().unwrap(), value);

        assert_eq!(
            encode!(Value::Dictionary {
                is_weak: false,
                entries: vec![]
            }),
            include_bytes!("../../test_data/amf3-empty-dictionary.bin")
        );
    }
}
