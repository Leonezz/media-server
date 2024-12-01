use core::time;
use std::io;

use crate::error::{AmfEncodeError, AmfEncodeResult};
use byteorder::{BigEndian, WriteBytesExt};

use super::{
    Value,
    amf3_marker::{self},
};

pub struct Encoder<W> {
    inner: W,
}

impl<W> Encoder<W> {
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

impl<W> Encoder<W>
where
    W: io::Write,
{
    pub fn new(inner: W) -> Self {
        Self { inner }
    }
    pub fn encode(&mut self, value: &Value) -> AmfEncodeResult<()> {
        match *value {
            Value::Undefined => self.encode_undefined(),
            Value::Null => self.encode_null(),
            Value::Boolean(value) => {
                if value {
                    self.encode_true()
                } else {
                    self.encode_false()
                }
            }
            Value::Integer(value) => self.encode_integer(value),
            Value::Double(value) => self.encode_double(value),
            Value::String(ref value) => self.encode_string(value),
            Value::XMLDocument(ref value) => self.encode_xml_document(value),
            Value::Date { millis_timestamp } => self.encode_date(millis_timestamp),
            Value::Array {
                ref assoc_entries,
                ref dense_entries,
            } => self.encode_array(assoc_entries, dense_entries),
            Value::Object {
                ref name,
                sealed_fields_count,
                ref entries,
            } => self.encode_object(name, sealed_fields_count, entries),
            Value::XML(ref value) => self.encode_xml(value),
            Value::ByteArray(ref value) => self.encode_byte_array(value),
            Value::I32Vector {
                is_fixed,
                ref entries,
            } => self.encode_i32_vector(is_fixed, entries),
            Value::U32Vector {
                is_fixed,
                ref entries,
            } => self.encode_u32_vector(is_fixed, entries),
            Value::DoubleVector {
                is_fixed,
                ref entries,
            } => self.encode_double_vector(is_fixed, entries),
            Value::ObjectVector {
                is_fixed,
                ref class_name,
                ref entries,
            } => self.encode_object_vector(class_name, is_fixed, entries),
            Value::Dictionary {
                is_weak,
                ref entries,
            } => self.encode_dictionary(is_weak, entries),
        }
    }

    fn encode_undefined(&mut self) -> AmfEncodeResult<()> {
        self.inner.write_u8(amf3_marker::UNDEFINED)?;
        Ok(())
    }

    fn encode_null(&mut self) -> AmfEncodeResult<()> {
        self.inner.write_u8(amf3_marker::NULL)?;
        Ok(())
    }

    fn encode_false(&mut self) -> AmfEncodeResult<()> {
        self.inner.write_u8(amf3_marker::FALSE)?;
        Ok(())
    }

    fn encode_true(&mut self) -> AmfEncodeResult<()> {
        self.inner.write_u8(amf3_marker::TRUE)?;
        Ok(())
    }

    fn write_u29(&mut self, u29: u32) -> AmfEncodeResult<()> {
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
            _ => return Err(AmfEncodeError::U29OutOfRange { value: u29 }),
        }
        Ok(())
    }

    fn encode_integer(&mut self, value: i32) -> AmfEncodeResult<()> {
        self.inner.write_u8(amf3_marker::INTEGER)?;
        let u29 = if value >= 0 {
            value as u32
        } else {
            ((1 << 29) + value) as u32
        };
        self.write_u29(u29)?;
        Ok(())
    }

    fn encode_double(&mut self, value: f64) -> AmfEncodeResult<()> {
        self.inner.write_u8(amf3_marker::DOUBLE)?;
        self.inner.write_f64::<BigEndian>(value)?;
        Ok(())
    }

    fn write_size(&mut self, size: usize) -> AmfEncodeResult<()> {
        if size >= (1 << 28) {
            return Err(AmfEncodeError::SizeOutOfRange { value: size });
        }
        let not_reference_bit = 1;
        self.write_u29(((size << 1) | not_reference_bit) as u32)?;
        Ok(())
    }
    fn write_utf8(&mut self, value: &str) -> AmfEncodeResult<()> {
        self.write_size(value.len())?;
        self.inner.write_all(value.as_bytes())?;
        Ok(())
    }
    fn encode_string(&mut self, value: &str) -> AmfEncodeResult<()> {
        self.inner.write_u8(amf3_marker::STRING)?;
        self.write_utf8(value)?;
        Ok(())
    }
    fn encode_xml_document(&mut self, xml_doc: &str) -> AmfEncodeResult<()> {
        self.inner.write_u8(amf3_marker::XML_DOCUMENT)?;
        self.write_utf8(xml_doc)?;
        Ok(())
    }

    fn encode_date(&mut self, millis_timestamp: time::Duration) -> AmfEncodeResult<()> {
        self.inner.write_u8(amf3_marker::DATE)?;
        self.write_size(0)?;
        self.inner
            .write_f64::<BigEndian>(millis_timestamp.as_millis() as f64)?;
        Ok(())
    }

    fn write_pairs(&mut self, pairs: &[(String, Value)]) -> AmfEncodeResult<()> {
        for (key, value) in pairs {
            self.write_utf8(&key)?;
            self.encode(value)?;
        }
        self.write_utf8("")?;
        Ok(())
    }
    fn encode_array(&mut self, assoc: &[(String, Value)], dense: &[Value]) -> AmfEncodeResult<()> {
        self.inner.write_u8(amf3_marker::ARRAY)?;
        self.write_size(dense.len())?;
        self.write_pairs(assoc)?;
        dense
            .iter()
            .map(|value| self.encode(value))
            .collect::<AmfEncodeResult<Vec<_>>>()?;
        Ok(())
    }

    fn write_trait(
        &mut self,
        class_name: &Option<String>,
        entries: &[(String, Value)],
        sealed_count: usize,
    ) -> AmfEncodeResult<()> {
        if sealed_count > entries.len() {
            return Err(AmfEncodeError::Amf3TraitInvalid {
                entries: Vec::from(entries),
                sealed_count: sealed_count,
            });
        }
        let not_reference_bit = 1 as usize;
        let is_externalizable = false as usize;
        let is_dynamic = (sealed_count < entries.len()) as usize;
        let u28 =
            (sealed_count << 3) | (is_dynamic << 2) | (is_externalizable << 1) | not_reference_bit;
        self.write_size(u28)?;
        let class_name = class_name.as_ref().map_or("", |s| s);
        self.write_utf8(&class_name)?;
        for (key, _) in entries.iter().take(sealed_count) {
            self.write_utf8(key)?;
        }
        Ok(())
    }

    fn encode_object(
        &mut self,
        class_name: &Option<String>,
        sealed_count: usize,
        entries: &[(String, Value)],
    ) -> AmfEncodeResult<()> {
        self.inner.write_u8(amf3_marker::OBJECT)?;
        self.write_trait(class_name, entries, sealed_count)?;
        for (_, value) in entries.iter().take(sealed_count) {
            self.encode(value)?;
        }
        if entries.len() > sealed_count {
            self.write_pairs(&entries[sealed_count..])?;
        }
        Ok(())
    }

    fn encode_xml(&mut self, xml: &str) -> AmfEncodeResult<()> {
        self.inner.write_u8(amf3_marker::XML)?;
        self.write_utf8(xml)?;
        Ok(())
    }

    fn encode_byte_array(&mut self, bytes: &[u8]) -> AmfEncodeResult<()> {
        self.inner.write_u8(amf3_marker::BYTE_ARRAY)?;
        self.write_size(bytes.len())?;
        self.inner.write_all(bytes)?;
        Ok(())
    }

    fn encode_i32_vector(&mut self, is_fixed: bool, value: &[i32]) -> AmfEncodeResult<()> {
        self.inner.write_u8(amf3_marker::VECTOR_INT)?;
        self.write_size(value.len())?;
        self.inner.write_u8(is_fixed as u8)?;
        for &v in value {
            self.inner.write_i32::<BigEndian>(v)?;
        }
        Ok(())
    }

    fn encode_u32_vector(&mut self, is_fixed: bool, value: &[u32]) -> AmfEncodeResult<()> {
        self.inner.write_u8(amf3_marker::VECTOR_UINT)?;
        self.write_size(value.len())?;
        self.inner.write_u8(is_fixed as u8)?;
        for &v in value {
            self.inner.write_u32::<BigEndian>(v)?;
        }
        Ok(())
    }

    fn encode_double_vector(&mut self, is_fixed: bool, value: &[f64]) -> AmfEncodeResult<()> {
        self.inner.write_u8(amf3_marker::VECTOR_DOUBLE)?;
        self.write_size(value.len())?;
        self.inner.write_u8(is_fixed as u8)?;
        for &v in value {
            self.inner.write_f64::<BigEndian>(v)?;
        }
        Ok(())
    }

    fn encode_object_vector(
        &mut self,
        class_name: &Option<String>,
        is_fixed: bool,
        value: &[Value],
    ) -> AmfEncodeResult<()> {
        self.inner.write_u8(amf3_marker::VECTOR_OBJECT)?;
        self.write_size(value.len())?;
        self.inner.write_u8(is_fixed as u8)?;
        self.write_utf8(class_name.as_ref().map_or("", |s| s))?;
        for v in value {
            self.encode(v)?;
        }
        Ok(())
    }

    fn encode_dictionary(
        &mut self,
        is_weak: bool,
        entries: &[(Value, Value)],
    ) -> AmfEncodeResult<()> {
        self.inner.write_u8(amf3_marker::DICTIONARY)?;
        self.write_size(entries.len())?;
        self.inner.write_u8(is_weak as u8)?;
        for (key, value) in entries {
            self.encode(key)?;
            self.encode(value)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use core::time;

    use crate::amf3::{self, Value};

    use super::Encoder;
    macro_rules! encode {
        ($value:expr) => {{
            let mut buf = Vec::new();
            let res = Encoder::new(&mut buf).encode(&$value);
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
        amf3::Encoder::new(&mut buf).encode(&value).unwrap();
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
