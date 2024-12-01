use core::time;
use std::io;

use crate::error::{AmfDecodeError, AmfDecodeResult};
use byteorder::{BigEndian, ReadBytesExt};

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
pub struct Decoder<R> {
    inner: R,
    referenceable: Amf3Referenceable,
}

impl<R> Decoder<R> {
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

impl<R> Decoder<R>
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
    pub fn decode(&mut self) -> AmfDecodeResult<Value> {
        let marker = self.inner.read_u8()?;
        match marker {
            amf3_marker::UNDEFINED => Ok(Value::Undefined),
            amf3_marker::NULL => Ok(Value::Null),
            amf3_marker::FALSE => Ok(Value::Boolean(false)),
            amf3_marker::TRUE => Ok(Value::Boolean(true)),
            amf3_marker::INTEGER => self.decode_integer(),
            amf3_marker::DOUBLE => self.decode_double(),
            amf3_marker::STRING => self.decode_string(),
            amf3_marker::XML_DOCUMENT => self.decode_xml_document(),
            amf3_marker::DATE => self.decode_date(),
            amf3_marker::ARRAY => self.decode_array(),
            amf3_marker::OBJECT => self.decode_object(),
            amf3_marker::XML => self.decode_xml(),
            amf3_marker::BYTE_ARRAY => self.decode_byte_array(),
            amf3_marker::VECTOR_INT => self.decode_i32_vector(),
            amf3_marker::VECTOR_UINT => self.decode_u32_vector(),
            amf3_marker::VECTOR_DOUBLE => self.decode_double_vector(),
            amf3_marker::VECTOR_OBJECT => self.decode_object_vector(),
            amf3_marker::DICTIONARY => self.decode_dictionary(),
            _ => Err(AmfDecodeError::Unknown { marker }),
        }
    }
    fn read_u29(&mut self) -> AmfDecodeResult<u32> {
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
    fn read_size_or_index(&mut self) -> AmfDecodeResult<SizeOrIndex> {
        let u29 = self.read_u29()? as usize;
        let is_index = (u29 & 0b01) == 0;
        let value = u29 >> 1;
        if is_index {
            Ok(SizeOrIndex::Index(value))
        } else {
            Ok(SizeOrIndex::Size(value))
        }
    }
    fn read_bytes(&mut self, len: usize) -> AmfDecodeResult<Vec<u8>> {
        let mut buf = vec![0; len];
        self.inner.read_exact(&mut buf)?;
        Ok(buf)
    }
    fn read_utf8(&mut self, len: usize) -> AmfDecodeResult<String> {
        let buf = self.read_bytes(len)?;
        let str = String::from_utf8(buf)?;
        Ok(str)
    }
    fn read_and_record_utf8(&mut self) -> AmfDecodeResult<String> {
        match self.read_size_or_index()? {
            SizeOrIndex::Index(index) => {
                let result = self
                    .referenceable
                    .strings
                    .get(index)
                    .ok_or(AmfDecodeError::OutOfRangeReference { index: index })?;
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
    fn read_and_record_object<F>(&mut self, f: F) -> AmfDecodeResult<Value>
    where
        F: FnOnce(&mut Self, usize) -> AmfDecodeResult<Value>,
    {
        match self.read_size_or_index()? {
            SizeOrIndex::Index(index) => self
                .referenceable
                .objects
                .get(index)
                .ok_or(AmfDecodeError::OutOfRangeReference { index: index })
                .and_then(|v| {
                    if *v == Value::Null {
                        Err(AmfDecodeError::CircularReference { index: index })
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

    fn read_trait(&mut self, size: usize) -> AmfDecodeResult<Amf3Trait> {
        if (size & 0b1) == 0 {
            let index = (size >> 1) as usize;
            let result = self
                .referenceable
                .traits
                .get(index)
                .ok_or(AmfDecodeError::OutOfRangeReference { index })?;
            return Ok(result.clone());
        }

        if (size & 0b10) != 0 {
            let class_name = self.read_and_record_utf8()?;
            return Err(AmfDecodeError::UnsupportedExternalizable { name: class_name });
        }

        let is_dynamic = (size & 0b100) != 0;
        let field_num = size >> 3;
        let class_name = self.read_and_record_utf8()?;
        let fields = (0..field_num)
            .map(|_| self.read_and_record_utf8())
            .collect::<AmfDecodeResult<_>>()?;
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

    fn read_and_record_trait(&mut self, size: usize) -> AmfDecodeResult<Amf3Trait> {
        let result = self.read_trait(size)?;
        self.referenceable.traits.push(result.clone());
        Ok(result)
    }

    fn decode_integer(&mut self) -> AmfDecodeResult<Value> {
        let result = self.read_u29()? as i32;
        let result = if result >= (1 << 28) {
            result - (1 << 29)
        } else {
            result
        };
        Ok(Value::Integer(result))
    }
    fn decode_double(&mut self) -> AmfDecodeResult<Value> {
        let result = self.inner.read_f64::<BigEndian>()?;
        Ok(Value::Double(result))
    }
    fn decode_string(&mut self) -> AmfDecodeResult<Value> {
        let str = self.read_and_record_utf8()?;
        Ok(Value::String(str))
    }
    fn decode_xml_document(&mut self) -> AmfDecodeResult<Value> {
        self.read_and_record_object(|this, size| this.read_utf8(size).map(Value::XMLDocument))
    }
    fn decode_date(&mut self) -> AmfDecodeResult<Value> {
        self.read_and_record_object(|this, _| {
            let millis_timestamp = this.inner.read_f64::<BigEndian>()?;
            if !(millis_timestamp.is_finite() && millis_timestamp.is_sign_positive()) {
                Err(AmfDecodeError::InvalidDate {
                    milliseconds: millis_timestamp,
                })
            } else {
                Ok(Value::Date {
                    millis_timestamp: time::Duration::from_millis(millis_timestamp as u64),
                })
            }
        })
    }
    fn read_pairs(&mut self) -> AmfDecodeResult<Vec<(String, Value)>> {
        let mut result = Vec::new();
        loop {
            let key = self.read_and_record_utf8()?;
            if key.is_empty() {
                return Ok(result);
            }
            let value = self.decode()?;
            result.push((key, value));
        }
    }
    fn decode_array(&mut self) -> AmfDecodeResult<Value> {
        self.read_and_record_object(|this, size| {
            let assoc_entries = this.read_pairs()?;
            let dense_entries = (0..size)
                .map(|_| this.decode())
                .collect::<AmfDecodeResult<_>>()?;
            Ok(Value::Array {
                assoc_entries,
                dense_entries,
            })
        })
    }
    fn decode_object(&mut self) -> AmfDecodeResult<Value> {
        self.read_and_record_object(|this, size| {
            let amf3_trait = this.read_and_record_trait(size)?;
            let mut entries = amf3_trait
                .fields
                .iter()
                .map(|key| {
                    let value = this.decode()?;
                    Ok((key.clone(), value))
                })
                .collect::<AmfDecodeResult<Vec<_>>>()?;
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
    fn decode_xml(&mut self) -> AmfDecodeResult<Value> {
        self.read_and_record_object(|this, len| this.read_utf8(len).map(Value::XML))
    }
    fn decode_byte_array(&mut self) -> AmfDecodeResult<Value> {
        self.read_and_record_object(|this, len| this.read_bytes(len).map(Value::ByteArray))
    }
    fn decode_i32_vector(&mut self) -> AmfDecodeResult<Value> {
        self.read_and_record_object(|this, count| {
            let is_fixed = this.inner.read_u8()? != 0;
            let entries = (0..count)
                .map(|_| this.inner.read_i32::<BigEndian>())
                .collect::<Result<_, _>>()?;
            Ok(Value::I32Vector { is_fixed, entries })
        })
    }
    fn decode_u32_vector(&mut self) -> AmfDecodeResult<Value> {
        self.read_and_record_object(|this, count| {
            let is_fixed = this.inner.read_u8()? != 0;
            let entries = (0..count)
                .map(|_| this.inner.read_u32::<BigEndian>())
                .collect::<Result<_, _>>()?;
            Ok(Value::U32Vector { is_fixed, entries })
        })
    }
    fn decode_double_vector(&mut self) -> AmfDecodeResult<Value> {
        self.read_and_record_object(|this, count| {
            let is_fixed = this.inner.read_u8()? != 0;
            let entries = (0..count)
                .map(|_| this.inner.read_f64::<BigEndian>())
                .collect::<Result<_, _>>()?;
            Ok(Value::DoubleVector { is_fixed, entries })
        })
    }
    fn decode_object_vector(&mut self) -> AmfDecodeResult<Value> {
        self.read_and_record_object(|this, count| {
            let is_fixed = this.inner.read_u8()? != 0;
            let class_name = this.read_and_record_utf8()?;
            let entries = (0..count)
                .map(|_| this.decode())
                .collect::<AmfDecodeResult<_>>()?;
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
    fn decode_dictionary(&mut self) -> AmfDecodeResult<Value> {
        self.read_and_record_object(|this, count| {
            let is_weak = this.inner.read_u8()? == 1;
            let entries = (0..count)
                .map(|_| Ok((this.decode()?, this.decode()?)))
                .collect::<AmfDecodeResult<_>>()?;
            Ok(Value::Dictionary { is_weak, entries })
        })
    }
}

#[cfg(test)]
mod tests {
    use core::{f64, time};
    use std::{
        io::{self},
        vec,
    };

    use crate::{amf3::Value, error::AmfDecodeError};

    use super::Decoder;
    macro_rules! decode {
        ($file:expr) => {{
            let data = include_bytes!($file);
            Decoder::new(&mut &data[..]).decode()
        }};
    }

    macro_rules! assert_eof {
        ($file:expr) => {
            let err = decode!($file).unwrap_err();
            match err {
                AmfDecodeError::Io(e) => assert_eq!(e.kind(), io::ErrorKind::UnexpectedEof),
                _ => assert!(false),
            }
        };
    }

    #[test]
    fn undefined() {
        assert_eq!(
            decode!("../../test_data/amf3-undefined.bin").unwrap(),
            Value::Undefined
        );
    }

    #[test]
    fn null() {
        assert_eq!(
            decode!("../../test_data/amf3-null.bin").unwrap(),
            Value::Null
        );
    }

    #[test]
    fn boolean() {
        assert_eq!(
            decode!("../../test_data/amf3-false.bin").unwrap(),
            Value::Boolean(false)
        );
        assert_eq!(
            decode!("../../test_data/amf3-true.bin").unwrap(),
            Value::Boolean(true)
        );
    }

    #[test]
    fn integer() {
        assert_eq!(
            decode!("../../test_data/amf3-0.bin").unwrap(),
            Value::Integer(0)
        );
        assert_eq!(
            decode!("../../test_data/amf3-min.bin").unwrap(),
            Value::Integer(-0x1000_0000)
        );
        assert_eq!(
            decode!("../../test_data/amf3-max.bin").unwrap(),
            Value::Integer(0x0FFF_FFFF)
        );
        assert_eq!(
            decode!("../../test_data/amf3-integer-2byte.bin").unwrap(),
            Value::Integer(0b1000_0000)
        );
        assert_eq!(
            decode!("../../test_data/amf3-integer-3byte.bin").unwrap(),
            Value::Integer(0b100_0000_0000_0000)
        );
    }

    #[test]
    fn double() {
        assert_eq!(
            decode!("../../test_data/amf3-float.bin").unwrap(),
            Value::Double(3.5)
        );
        assert_eq!(
            decode!("../../test_data/amf3-bignum.bin").unwrap(),
            Value::Double(2f64.powf(1000f64))
        );
        assert_eq!(
            decode!("../../test_data/amf3-large-min.bin").unwrap(),
            Value::Double(-0x1000_0001 as f64)
        );
        assert_eq!(
            decode!("../../test_data/amf3-large-max.bin").unwrap(),
            Value::Double(268_435_456_f64)
        );
        assert_eq!(
            decode!("../../test_data/amf3-double-positive-infinity.bin").unwrap(),
            Value::Double(f64::INFINITY)
        );

        assert_eof!("../../test_data/amf3-double-partial.bin");
    }

    #[test]
    fn string() {
        assert_eq!(
            decode!("../../test_data/amf3-string.bin").unwrap(),
            Value::String("String . String".to_string())
        );
        assert_eq!(
            decode!("../../test_data/amf3-symbol.bin").unwrap(),
            Value::String("foo".to_string())
        );

        assert_eq!(
            decode!("../../test_data/amf3-string-ref.bin").unwrap(),
            Value::Array {
                assoc_entries: Vec::new(),
                dense_entries: vec![
                    Value::String("foo".to_string()),
                    Value::String("str".to_string()),
                    Value::String("foo".to_string()),
                    Value::String("str".to_string()),
                    Value::String("foo".to_string()),
                    Value::Object {
                        name: None,
                        sealed_fields_count: 0,
                        entries: vec![("str".to_string(), Value::String("foo".to_string()))]
                    }
                ]
            }
        );
        assert_eq!(
            decode!("../../test_data/amf3-encoded-string-ref.bin").unwrap(),
            Value::Array {
                assoc_entries: Vec::new(),
                dense_entries: vec![
                    Value::String("this is a テスト".to_string()),
                    Value::String("this is a テスト".to_string())
                ]
            }
        );

        assert_eq!(
            decode!("../../test_data/amf3-complex-encoded-string-array.bin").unwrap(),
            Value::Array {
                assoc_entries: Vec::new(),
                dense_entries: vec![
                    Value::Integer(5),
                    Value::String("Shift テスト".to_string()),
                    Value::String("UTF テスト".to_string()),
                    Value::Integer(5)
                ]
            }
        );

        assert_eq!(
            decode!("../../test_data/amf3-empty-string-ref.bin").unwrap(),
            Value::Array {
                assoc_entries: Vec::new(),
                dense_entries: vec![Value::String("".to_string()), Value::String("".to_string())]
            }
        );

        assert_eof!("../../test_data/amf3-string-partial.bin");
    }

    #[test]
    fn xml_document() {
        assert_eq!(
            decode!("../../test_data/amf3-xml-doc.bin").unwrap(),
            Value::XMLDocument("<parent><child prop=\"test\" /></parent>".to_string())
        )
    }

    #[test]
    fn date() {
        let date = Value::Date {
            millis_timestamp: time::Duration::from_secs(0),
        };
        assert_eq!(decode!("../../test_data/amf3-date.bin").unwrap(), date);
        assert_eq!(
            decode!("../../test_data/amf3-date-ref.bin").unwrap(),
            Value::Array {
                assoc_entries: Vec::new(),
                dense_entries: vec![date.clone(), date]
            }
        );
        assert_eof!("../../test_data/amf3-date-partial.bin");

        assert!(matches!(
            decode!("../../test_data/amf3-date-invalid-millis.bin").unwrap_err(),
            AmfDecodeError::InvalidDate {
                milliseconds: f64::INFINITY
            }
        ));
        assert!(matches!(
            decode!("../../test_data/amf3-date-minus-millis.bin").unwrap_err(),
            AmfDecodeError::InvalidDate { milliseconds: -1.0 }
        ));
    }

    #[test]
    fn array() {
        assert_eq!(
            decode!("../../test_data/amf3-primitive-array.bin").unwrap(),
            Value::Array {
                assoc_entries: Vec::new(),
                dense_entries: vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(3),
                    Value::Integer(4),
                    Value::Integer(5)
                ]
            }
        );
        assert_eq!(
            decode!("../../test_data/amf3-array-ref.bin").unwrap(),
            Value::Array {
                assoc_entries: Vec::new(),
                dense_entries: vec![
                    Value::Array {
                        assoc_entries: Vec::new(),
                        dense_entries: vec![
                            Value::Integer(1),
                            Value::Integer(2),
                            Value::Integer(3)
                        ]
                    },
                    Value::Array {
                        assoc_entries: Vec::new(),
                        dense_entries: vec![
                            Value::String("a".to_string()),
                            Value::String("b".to_string()),
                            Value::String("c".to_string())
                        ]
                    },
                    Value::Array {
                        assoc_entries: Vec::new(),
                        dense_entries: vec![
                            Value::Integer(1),
                            Value::Integer(2),
                            Value::Integer(3)
                        ]
                    },
                    Value::Array {
                        assoc_entries: Vec::new(),
                        dense_entries: vec![
                            Value::String("a".to_string()),
                            Value::String("b".to_string()),
                            Value::String("c".to_string())
                        ]
                    }
                ]
            }
        );

        assert_eq!(
            decode!("../../test_data/amf3-associative-array.bin").unwrap(),
            Value::Array {
                assoc_entries: vec![
                    ("2".to_string(), Value::String("bar3".to_string())),
                    ("foo".to_string(), Value::String("bar".to_string())),
                    ("asdf".to_string(), Value::String("fdsa".to_string()))
                ],
                dense_entries: vec![
                    Value::String("bar".to_string()),
                    Value::String("bar1".to_string()),
                    Value::String("bar2".to_string())
                ]
            }
        );

        {
            let o1 = Value::Object {
                name: None,
                sealed_fields_count: 0,
                entries: vec![("foo_one".to_string(), Value::String("bar_one".to_string()))],
            };
            let o2 = Value::Object {
                name: None,
                sealed_fields_count: 0,
                entries: vec![("foo_two".to_string(), Value::String("".to_string()))],
            };
            let o3 = Value::Object {
                name: None,
                sealed_fields_count: 0,
                entries: vec![("foo_three".to_string(), Value::Integer(42))],
            };
            let empty = Value::Object {
                name: None,
                sealed_fields_count: 0,
                entries: vec![],
            };
            assert_eq!(
                decode!("../../test_data/amf3-mixed-array.bin").unwrap(),
                Value::Array {
                    assoc_entries: Vec::new(),
                    dense_entries: vec![
                        o1.clone(),
                        o2.clone(),
                        o3.clone(),
                        empty.clone(),
                        Value::Array {
                            assoc_entries: Vec::new(),
                            dense_entries: vec![o1.clone(), o2.clone(), o3.clone()]
                        },
                        Value::Array {
                            assoc_entries: Vec::new(),
                            dense_entries: Vec::new()
                        },
                        Value::Integer(42),
                        Value::String("".to_string()),
                        Value::Array {
                            assoc_entries: Vec::new(),
                            dense_entries: Vec::new()
                        },
                        Value::String("".to_string()),
                        empty.clone(),
                        Value::String("bar_one".to_string()),
                        o3.clone()
                    ]
                }
            );
        }
    }

    #[test]
    fn object() {
        let o = Value::Object {
            name: None,
            sealed_fields_count: 0,
            entries: vec![("foo".to_string(), Value::String("bar".to_string()))],
        };

        assert_eq!(
            decode!("../../test_data/amf3-object-ref.bin").unwrap(),
            Value::Array {
                assoc_entries: Vec::new(),
                dense_entries: vec![
                    Value::Array {
                        assoc_entries: Vec::new(),
                        dense_entries: vec![o.clone(), o.clone()]
                    },
                    Value::String("bar".to_string()),
                    Value::Array {
                        assoc_entries: Vec::new(),
                        dense_entries: vec![o.clone(), o.clone()]
                    }
                ]
            }
        );

        assert_eq!(
            decode!("../../test_data/amf3-dynamic-object.bin").unwrap(),
            Value::Object {
                name: None,
                sealed_fields_count: 0,
                entries: vec![
                    ("property_one".to_string(), Value::String("foo".to_string())),
                    (
                        "another_public_property".to_string(),
                        Value::String("a_public_value".to_string())
                    ),
                    ("nil_property".to_string(), Value::Null)
                ]
            }
        );

        assert_eq!(
            decode!("../../test_data/amf3-typed-object.bin").unwrap(),
            Value::Object {
                name: Some("org.amf.ASClass".to_string()),
                sealed_fields_count: 2,
                entries: vec![
                    ("foo".to_string(), Value::String("bar".to_string())),
                    ("baz".to_string(), Value::Null)
                ]
            }
        );

        let o = [
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
        ];
        assert_eq!(
            decode!("../../test_data/amf3-trait-ref.bin").unwrap(),
            Value::Array {
                assoc_entries: Vec::new(),
                dense_entries: Vec::from(o)
            }
        );

        assert_eq!(
            decode!("../../test_data/amf3-hash.bin").unwrap(),
            Value::Object {
                name: None,
                sealed_fields_count: 0,
                entries: vec![
                    ("foo".to_string(), Value::String("bar".to_string())),
                    ("answer".to_string(), Value::Integer(42))
                ]
            }
        );

        assert!(matches!(
            decode!("../../test_data/amf3-externalizable.bin").unwrap_err(),
            AmfDecodeError::UnsupportedExternalizable {
                name
            } if name == "ExternalizableTest".to_string()
        ));

        assert!(matches!(
            decode!("../../test_data/amf3-array-collection.bin").unwrap_err(),
            AmfDecodeError::UnsupportedExternalizable { name } if name == "flex.messaging.io.ArrayCollection".to_string()
        ));
    }

    #[test]
    fn xml() {
        assert_eq!(
            decode!("../../test_data/amf3-xml.bin").unwrap(),
            Value::XML("<parent><child prop=\"test\"/></parent>".to_string())
        );
        assert_eq!(
            decode!("../../test_data/amf3-xml-ref.bin").unwrap(),
            Value::Array {
                assoc_entries: Vec::new(),
                dense_entries: vec![
                    Value::XML("<parent><child prop=\"test\"/></parent>".to_string()),
                    Value::XML("<parent><child prop=\"test\"/></parent>".to_string())
                ]
            }
        );

        assert_eof!("../../test_data/amf3-xml-partial.bin");
    }

    #[test]
    fn byte_array() {
        assert_eq!(
            decode!("../../test_data/amf3-byte-array.bin").unwrap(),
            Value::ByteArray(vec![
                0, 3, 227, 129, 147, 227, 130, 140, 116, 101, 115, 116, 64
            ])
        );

        assert_eq!(
            decode!("../../test_data/amf3-byte-array-ref.bin").unwrap(),
            Value::Array {
                assoc_entries: Vec::new(),
                dense_entries: vec![
                    Value::ByteArray(b"ASDF".to_vec()),
                    Value::ByteArray(b"ASDF".to_vec())
                ]
            }
        );
    }

    #[test]
    fn i32_vector() {
        assert_eq!(
            decode!("../../test_data/amf3-vector-int.bin").unwrap(),
            Value::I32Vector {
                is_fixed: false,
                entries: vec![4, -20, 12]
            }
        );

        assert_eof!("../../test_data/amf3-vector-int-partial.bin");
    }

    #[test]
    fn u32_vector() {
        assert_eq!(
            decode!("../../test_data/amf3-vector-uint.bin").unwrap(),
            Value::U32Vector {
                is_fixed: false,
                entries: vec![4, 20, 12]
            }
        );

        assert_eof!("../../test_data/amf3-vector-uint-partial.bin");
    }

    #[test]
    fn double_vector() {
        assert_eq!(
            decode!("../../test_data/amf3-vector-double.bin").unwrap(),
            Value::DoubleVector {
                is_fixed: false,
                entries: vec![4.3, -20.6]
            }
        );

        assert_eof!("../../test_data/amf3-vector-partial.bin");
    }

    #[test]
    fn object_vector() {
        let os = vec![
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
        ];
        assert_eq!(
            decode!("../../test_data/amf3-vector-object.bin").unwrap(),
            Value::ObjectVector {
                is_fixed: false,
                class_name: Some("org.amf.ASClass".to_string()),
                entries: os
            }
        );
    }

    #[test]
    fn dictionary() {
        let entries = vec![
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
        ];

        assert_eq!(
            decode!("../../test_data/amf3-dictionary.bin").unwrap(),
            Value::Dictionary {
                is_weak: false,
                entries
            }
        );
        assert_eq!(
            decode!("../../test_data/amf3-empty-dictionary.bin").unwrap(),
            Value::Dictionary {
                is_weak: false,
                entries: vec![]
            }
        );

        assert_eof!("../../test_data/amf3-dictionary-partial.bin");
    }

    #[test]
    fn reference() {
        assert!(matches!(
            decode!("../../test_data/amf3-graph-member.bin").unwrap_err(),
            AmfDecodeError::CircularReference { index: 0 }
        ));

        assert!(matches!(
            decode!("../../test_data/amf3-bad-object-ref.bin").unwrap_err(),
            AmfDecodeError::OutOfRangeReference { index: 10 }
        ));

        assert!(matches!(
            decode!("../../test_data/amf3-bad-trait-ref.bin").unwrap_err(),
            AmfDecodeError::OutOfRangeReference { index: 4 }
        ));

        assert!(matches!(
            decode!("../../test_data/amf3-bad-string-ref.bin").unwrap_err(),
            AmfDecodeError::OutOfRangeReference { index: 8 }
        ));
    }

    #[test]
    fn unknown_marker() {
        assert!(matches!(
            decode!("../../test_data/amf3-unknown-marker.bin").unwrap_err(),
            AmfDecodeError::Unknown { marker: 123 }
        ));
    }

    #[test]
    fn empty() {
        assert_eof!("../../test_data/amf0-empty.bin");
    }

    #[test]
    fn u29() {
        assert_eof!("../../test_data/amf3-u29-partial.bin");
    }
}
