use core::time;
use std::{io, vec};

use byteorder::{BigEndian, ReadBytesExt};

use crate::errors::{AmfError, AmfResult};

use super::{Value, amf0_marker, amf3};

#[derive(Debug)]
struct Amf0Referenceable {
    objects: Vec<Value>,
}

#[derive(Debug)]
pub struct Reader<R> {
    inner: R,
    referenceable: Amf0Referenceable,
}
impl<R> Reader<R> {
    /// Unwraps this `Decoder`, returning the underlying reader.
    pub fn into_inner(self) -> R {
        self.inner
    }

    /// Get the reference to the underlying reader.
    pub fn inner(&self) -> &R {
        &self.inner
    }

    /// Get the mutable reference to the underlying reader.
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
            referenceable: Amf0Referenceable {
                objects: Vec::new(),
            },
        }
    }
    pub fn read(&mut self) -> AmfResult<Option<Value>> {
        let marker = self.inner.read_u8();
        if marker.is_err() {
            return Ok(None);
        }
        let marker = marker.expect("this cannot be err");
        let value = match marker {
            amf0_marker::NUMBER => self.read_number(),
            amf0_marker::BOOLEAN => self.read_boolean(),
            amf0_marker::STRING => self.read_string(),
            amf0_marker::OBJECT => self.read_anonymous_object(),
            amf0_marker::MOVIECLIP => Err(AmfError::Unsupported { marker }),
            amf0_marker::NULL => Ok(Value::Null),
            amf0_marker::UNDEFINED => Ok(Value::Undefined),
            amf0_marker::REFERENCE => self.read_reference(),
            amf0_marker::ECMA_ARRAY => self.read_ecma_array(),
            amf0_marker::OBJECT_END => Ok(Value::ObjectEnd),
            amf0_marker::STRICT_ARRAY => self.read_strict_array(),
            amf0_marker::DATE => self.read_date(),
            amf0_marker::LONG_STRING => self.read_long_string(),
            amf0_marker::UNSUPPORTED => Err(AmfError::Unsupported { marker }),
            amf0_marker::RECORDSET => Err(AmfError::Unsupported { marker }),
            amf0_marker::XML_DOCUMENT => self.read_xml_document(),
            amf0_marker::TYPED_OBJECT => self.read_typed_object(),
            amf0_marker::AVMPLUS_OBJECT => self.read_avm_plus(),
            _ => Err(AmfError::Unknown { marker }),
        };
        match value {
            Ok(v) => Ok(Some(v)),
            Err(err) => Err(err),
        }
    }

    pub fn read_all(&mut self) -> AmfResult<Vec<Value>> {
        let mut result = Vec::new();
        while let Ok(Some(value)) = self.read() {
            result.push(value);
        }
        Ok(result)
    }

    pub fn read_number(&mut self) -> AmfResult<Value> {
        let number = self.inner.read_f64::<BigEndian>()?;
        Ok(Value::Number(number))
    }
    pub fn read_boolean(&mut self) -> AmfResult<Value> {
        let bool = self.inner.read_u8()?;
        Ok(Value::Boolean(bool != 0))
    }
    fn read_utf8_inner(&mut self, len: usize) -> AmfResult<String> {
        let mut buffer = vec![0; len];
        self.inner.read_exact(&mut buffer)?;
        let result = String::from_utf8(buffer)?;
        Ok(result)
    }
    pub fn read_string(&mut self) -> AmfResult<Value> {
        let len = self.inner.read_u16::<BigEndian>()?;
        self.read_utf8_inner(len as usize).map(Value::String)
    }
    pub fn read_long_string(&mut self) -> AmfResult<Value> {
        let len = self.inner.read_u32::<BigEndian>()?;
        self.read_utf8_inner(len as usize).map(Value::String)
    }
    fn read_key_value_pairs_inner(&mut self) -> AmfResult<Vec<(String, Value)>> {
        let mut result: Vec<(String, Value)> = Vec::new();
        loop {
            let len: u16 = self.inner.read_u16::<BigEndian>()?;
            let key = self.read_utf8_inner(len as usize)?;
            match self.read() {
                Ok(Some(Value::ObjectEnd)) if key.is_empty() => {
                    break;
                }
                Ok(None) => {
                    return Err(AmfError::Io(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "unexpected eof",
                    )));
                }
                Ok(Some(value)) => {
                    result.push((key, value));
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }
        Ok(result)
    }
    pub fn read_anonymous_object(&mut self) -> AmfResult<Value> {
        self.read_and_record_referenceable_inner(|this| {
            let pairs = this.read_key_value_pairs_inner()?;
            Ok(Value::Object {
                name: None,
                entries: pairs,
            })
        })
    }
    pub fn read_reference(&mut self) -> AmfResult<Value> {
        let index = self.inner.read_u16::<BigEndian>()? as usize;
        self.referenceable
            .objects
            .get(index)
            .ok_or(AmfError::OutOfRangeReference { index })
            .and_then(|v| match *v {
                Value::Null => Err(AmfError::CircularReference { index }),
                _ => Ok(v.clone()),
            })
    }
    pub fn read_ecma_array(&mut self) -> AmfResult<Value> {
        self.read_and_record_referenceable_inner(|this| {
            // TODO - is this completely useless?
            let _len = this.inner.read_u32::<BigEndian>()? as usize;
            let pairs = this.read_key_value_pairs_inner()?;
            Ok(Value::ECMAArray(pairs))
        })
    }
    pub fn read_strict_array(&mut self) -> AmfResult<Value> {
        self.read_and_record_referenceable_inner(|this| {
            let len = this.inner.read_u32::<BigEndian>()? as usize;
            let values = (0..len)
                .map(|_| match this.read() {
                    Ok(None) => Err(AmfError::Io(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "expected eof",
                    ))),
                    Ok(Some(value)) => Ok(value),
                    Err(err) => Err(err),
                })
                .collect::<AmfResult<_>>()?;
            Ok(Value::StrictArray(values))
        })
    }
    pub fn read_date(&mut self) -> AmfResult<Value> {
        let timestamp = self.inner.read_f64::<BigEndian>()?;
        if !(timestamp.is_finite() && timestamp.is_sign_positive()) {
            return Err(AmfError::InvalidDate {
                milliseconds: timestamp,
            });
        }
        let time_zone = self.inner.read_i16::<BigEndian>()?;
        if time_zone != 0x0000 {
            return Err(AmfError::UnexpectedTimeZone { offset: time_zone });
        }
        Ok(Value::Date {
            time_zone,
            millis_timestamp: time::Duration::from_millis(timestamp as u64),
        })
    }
    pub fn read_xml_document(&mut self) -> AmfResult<Value> {
        let len = self.inner.read_u32::<BigEndian>()?;
        self.read_utf8_inner(len as usize).map(Value::XMLDocument)
    }
    pub fn read_typed_object(&mut self) -> AmfResult<Value> {
        self.read_and_record_referenceable_inner(|this| {
            let name_len = this.inner.read_u16::<BigEndian>()?;
            let name = this.read_utf8_inner(name_len as usize)?;
            let pairs = this.read_key_value_pairs_inner()?;
            Ok(Value::Object {
                name: Some(name),
                entries: pairs,
            })
        })
    }
    pub fn read_avm_plus(&mut self) -> AmfResult<Value> {
        let result = amf3::Reader::new(&mut self.inner).read()?;
        match result {
            Some(v) => Ok(Value::AVMPlus(v)),
            None => Err(AmfError::Io(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "unexpected eof",
            ))),
        }
    }
    fn read_and_record_referenceable_inner<F>(&mut self, f: F) -> AmfResult<Value>
    where
        F: FnOnce(&mut Self) -> AmfResult<Value>,
    {
        let len = self.referenceable.objects.len();
        self.referenceable.objects.push(Value::Null);
        let result = f(self)?;
        self.referenceable.objects[len] = result.clone();
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use core::{f64, time};
    use std::io::{self};

    use crate::{
        amf0::{Value, amf0_marker},
        amf3,
        errors::AmfError,
    };

    use super::Reader;
    macro_rules! decode {
        ($file:expr) => {{
            let data = include_bytes!($file);
            Reader::new(&mut &data[..]).read()
        }};
    }

    macro_rules! assert_eof {
        ($file:expr) => {
            let err = decode!($file).unwrap_err();
            match err {
                AmfError::Io(e) => assert_eq!(e.kind(), io::ErrorKind::UnexpectedEof),
                _ => assert!(false),
            }
        };
    }
    #[test]
    fn number() {
        assert_eq!(
            decode!("../../test_data/amf0-number.bin").unwrap().unwrap(),
            Value::Number(3.5)
        );
        assert_ne!(
            decode!("../../test_data/amf0-number.bin").unwrap().unwrap(),
            Value::Number(1.)
        );
        assert_ne!(
            decode!("../../test_data/amf0-number.bin").unwrap().unwrap(),
            Value::Null
        );
        assert_eq!(
            decode!("../../test_data/amf0-number-negative-infinity.bin")
                .unwrap()
                .unwrap(),
            Value::Number(f64::NEG_INFINITY)
        );
        assert_eq!(
            decode!("../../test_data/amf0-number-positive-infinity.bin")
                .unwrap()
                .unwrap(),
            Value::Number(f64::INFINITY)
        );

        assert_eof!("../../test_data/amf0-number-partial.bin");

        let is_nan = |v| match v {
            Value::Number(inner) => f64::is_nan(inner),
            _ => false,
        };
        assert!(is_nan(
            decode!("../../test_data/amf0-number-quiet-nan.bin")
                .unwrap()
                .unwrap()
        ));
        assert!(is_nan(
            decode!("../../test_data/amf0-number-signaling-nan.bin")
                .unwrap()
                .unwrap()
        ));
    }

    #[test]
    fn boolean() {
        assert_eq!(
            decode!("../../test_data/amf0-boolean-true.bin")
                .unwrap()
                .unwrap(),
            Value::Boolean(true)
        );
        assert_eq!(
            decode!("../../test_data/amf0-boolean-false.bin")
                .unwrap()
                .unwrap(),
            Value::Boolean(false)
        );

        assert_eof!("../../test_data/amf0-boolean-partial.bin");
    }

    #[test]
    fn string() {
        assert_eq!(
            decode!("../../test_data/amf0-string.bin").unwrap().unwrap(),
            Value::String("this is a テスト".to_string())
        );
        assert_ne!(
            decode!("../../test_data/amf0-string.bin").unwrap().unwrap(),
            Value::String("random utf8 字".to_string())
        );
        assert_eof!("../../test_data/amf0-strict-array-partial.bin");
    }

    #[test]
    fn long_string() {
        assert_eq!(
            decode!("../../test_data/amf0-long-string.bin")
                .unwrap()
                .unwrap(),
            Value::String("a".repeat(0x10013))
        );

        assert_eof!("../../test_data/amf0-long-string-partial.bin");
    }

    #[test]
    fn xml() {
        assert_eq!(
            decode!("../../test_data/amf0-xml-doc.bin")
                .unwrap()
                .unwrap(),
            Value::XMLDocument("<parent><child prop=\"test\" /></parent>".to_string())
        );

        assert_eof!("../../test_data/amf0-xml-document-partial.bin");
    }

    #[test]
    fn object() {
        {
            let pairs = vec![
                ("".to_string(), Value::String("".to_string())),
                ("foo".to_string(), Value::String("baz".to_string())),
                #[allow(clippy::approx_constant)]
                ("bar".to_string(), Value::Number(3.14)),
            ];
            let err = decode!("../../test_data/amf0-object.bin");
            println!("{:?}", err);
            assert_eq!(
                decode!("../../test_data/amf0-object.bin").unwrap().unwrap(),
                Value::Object {
                    name: None,
                    entries: pairs
                }
            )
        }

        assert_eof!("../../test_data/amf0-object-partial.bin");
    }

    #[test]
    fn movieclip() {
        let err = decode!("../../test_data/amf0-movieclip.bin").unwrap_err();
        match err {
            AmfError::Unsupported { marker } => assert_eq!(marker, amf0_marker::MOVIECLIP),
            _ => panic!("unexpected error: {:?}", err),
        }
    }

    #[test]
    fn null() {
        assert_eq!(
            decode!("../../test_data/amf0-null.bin").unwrap().unwrap(),
            Value::Null
        )
    }

    #[test]
    fn undefined() {
        assert_eq!(
            decode!("../../test_data/amf0-undefined.bin")
                .unwrap()
                .unwrap(),
            Value::Undefined
        )
    }

    #[test]
    fn reference() {
        let pairs = vec![
            ("foo".to_string(), Value::String("baz".to_string())),
            #[allow(clippy::approx_constant)]
            ("bar".to_string(), Value::Number(3.14)),
        ];
        let object = Value::Object {
            name: None,
            entries: pairs,
        };
        let reference_pairs = vec![("0".to_string(), object.clone()), ("1".to_string(), object)];

        assert_eq!(
            decode!("../../test_data/amf0-ref-test.bin")
                .unwrap()
                .unwrap(),
            Value::Object {
                name: None,
                entries: reference_pairs
            }
        );

        assert_eof!("../../test_data/amf0-object-partial.bin");
    }

    #[test]
    fn ecma_array() {
        {
            let arr = vec![
                ("0".to_string(), Value::String("a".to_string())),
                ("1".to_string(), Value::String("b".to_string())),
                ("2".to_string(), Value::String("c".to_string())),
                ("3".to_string(), Value::String("d".to_string())),
            ];
            assert_eq!(
                decode!("../../test_data/amf0-ecma-ordinal-array.bin")
                    .unwrap()
                    .unwrap(),
                Value::ECMAArray(arr)
            );
        }

        {
            let arr = vec![
                ("c".to_string(), Value::String("d".to_string())),
                ("a".to_string(), Value::String("b".to_string())),
            ];
            assert_eq!(
                decode!("../../test_data/amf0-hash.bin").unwrap().unwrap(),
                Value::ECMAArray(arr)
            );
        }

        assert_eof!("../../test_data/amf0-ecma-array-partial.bin");
    }

    #[test]
    fn strict_array() {
        let arr = vec![
            Value::Number(1.0),
            Value::String("2".to_string()),
            Value::Number(3.0),
        ];
        assert_eq!(
            decode!("../../test_data/amf0-strict-array.bin")
                .unwrap()
                .unwrap(),
            Value::StrictArray(arr)
        );

        assert_eof!("../../test_data/amf0-strict-array-partial.bin");
    }

    #[test]
    fn date() {
        assert_eq!(
            decode!("../../test_data/amf0-date.bin").unwrap().unwrap(),
            Value::Date {
                time_zone: 0,
                millis_timestamp: time::Duration::from_millis(1_590_796_800_000)
            }
        );
        assert_eq!(
            decode!("../../test_data/amf0-time.bin").unwrap().unwrap(),
            Value::Date {
                time_zone: 0,
                millis_timestamp: time::Duration::from_millis(1_045_112_400_000)
            }
        );
        assert!(matches!(
            decode!("../../test_data/amf0-date-minus.bin"),
            Err(AmfError::InvalidDate { milliseconds: -1.0 })
        ));
        assert!(matches!(
            decode!("../../test_data/amf0-date-invalid.bin"),
            Err(AmfError::InvalidDate {
                milliseconds: f64::INFINITY
            })
        ));

        assert_eof!("../../test_data/amf0-date-partial.bin");
    }

    #[test]
    fn typed_object() {
        let pairs = vec![
            ("foo".to_string(), Value::String("bar".to_string())),
            ("baz".to_string(), Value::Null),
        ];
        assert_eq!(
            decode!("../../test_data/amf0-typed-object.bin")
                .unwrap()
                .unwrap(),
            Value::Object {
                name: Some("org.amf.ASClass".to_string()),
                entries: pairs
            }
        );

        assert_eof!("../../test_data/amf0-typed-object-partial.bin");
    }

    #[test]
    fn unsupported() {
        assert!(matches!(
            decode!("../../test_data/amf0-movieclip.bin"),
            Err(AmfError::Unsupported {
                marker: amf0_marker::MOVIECLIP
            })
        ));
        assert!(matches!(
            decode!("../../test_data/amf0-recordset.bin"),
            Err(AmfError::Unsupported {
                marker: amf0_marker::RECORDSET
            })
        ));
        assert!(matches!(
            decode!("../../test_data/amf0-unsupported.bin"),
            Err(AmfError::Unsupported {
                marker: amf0_marker::UNSUPPORTED
            })
        ));
    }

    #[test]
    fn unknown() {
        assert_eof!("../../test_data/amf0-empty.bin");
        assert!(matches!(
            decode!("../../test_data/amf0-unknown-marker.bin"),
            Err(AmfError::Unknown { marker: _ })
        ));
    }

    #[test]
    fn avm_plus() {
        assert_eq!(
            decode!("../../test_data/amf0-avmplus-object.bin")
                .unwrap()
                .unwrap(),
            Value::AVMPlus(amf3::Value::Array {
                assoc_entries: vec![],
                dense_entries: (1..4).map(amf3::Value::Integer).collect()
            })
        );
    }
}
