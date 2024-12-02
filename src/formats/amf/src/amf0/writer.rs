use core::time;
use std::{collections::HashMap, io};

use crate::{amf3, errors::AmfWriteResult};

use byteorder::{BigEndian, WriteBytesExt};

use super::{Value, amf0_marker};

#[derive(Debug)]
pub struct Writer<W> {
    inner: W,
}

impl<W> Writer<W> {
    pub fn inner(&self) -> &W {
        &self.inner
    }
    pub fn inner_mut(&mut self) -> &mut W {
        &mut self.inner
    }
    pub fn into_inner(self) -> W {
        self.inner
    }
}

impl<W> Writer<W>
where
    W: io::Write,
{
    pub fn new(inner: W) -> Self {
        Self { inner }
    }
    pub fn write(&mut self, v: &Value) -> AmfWriteResult {
        match *v {
            Value::Number(n) => self.write_number(n),
            Value::Boolean(b) => self.write_boolean(b),
            Value::String(ref ss) => self.write_string(ss),
            Value::Object {
                ref name,
                ref entries,
            } => match name {
                Some(name) => self.write_typed_object_arr_inner(&name, &entries),
                None => self.write_anonymous_object_arr(&entries),
            },
            Value::Null => self.write_null(),
            Value::Undefined => self.write_undefined(),
            Value::Reference { index } => self.write_reference(index),
            Value::ECMAArray(ref arr) => self.write_ecma_array(arr),
            Value::ObjectEnd => self.write_object_end(),
            Value::StrictArray(ref arr) => self.write_strict_array(arr),
            Value::Date {
                time_zone,
                millis_timestamp: unix_timestamp,
            } => self.write_date(unix_timestamp, time_zone),
            Value::XMLDocument(ref xml) => self.write_xml(xml),
            Value::AVMPlus(ref value) => self.write_avm_plus(value),
        }
    }
    fn write_number(&mut self, v: f64) -> AmfWriteResult {
        self.inner.write_u8(amf0_marker::NUMBER)?;
        self.inner.write_f64::<BigEndian>(v)?;
        Ok(())
    }
    fn write_boolean(&mut self, v: bool) -> AmfWriteResult {
        self.inner.write_u8(amf0_marker::BOOLEAN)?;
        self.inner.write_u8(v as u8)?;
        Ok(())
    }
    fn write_short_string_inner(&mut self, v: &str) -> AmfWriteResult {
        assert!(v.len() < 0xFFFF); // TODO CHECK this
        self.inner.write_u16::<BigEndian>(v.len() as u16)?;
        self.inner.write_all(v.as_bytes())?;
        Ok(())
    }
    fn write_long_string_inner(&mut self, v: &str) -> AmfWriteResult {
        assert!(v.len() <= 0xFFFF_FFFF);
        self.inner.write_u32::<BigEndian>(v.len() as u32)?;
        self.inner.write_all(v.as_bytes())?;
        Ok(())
    }
    fn write_string(&mut self, v: &str) -> AmfWriteResult {
        if v.len() < 0xFFFF {
            self.inner.write_u8(amf0_marker::STRING)?;
            self.write_short_string_inner(v)?;
        } else {
            self.inner.write_u8(amf0_marker::LONG_STRING)?;
            self.write_long_string_inner(v)?;
        }
        Ok(())
    }
    fn write_pairs_inner(&mut self, entries: &[(String, Value)]) -> AmfWriteResult {
        for (key, value) in entries {
            self.write_short_string_inner(&key)?;
            self.write(value)?;
        }
        self.inner.write_u16::<BigEndian>(0)?;
        self.inner.write_u8(amf0_marker::OBJECT_END)?;
        Ok(())
    }
    fn write_anonymous_object_arr(&mut self, entries: &[(String, Value)]) -> AmfWriteResult {
        assert!(entries.len() <= 0xFFFF_FFFF);
        self.inner.write_u8(amf0_marker::OBJECT)?;
        self.write_pairs_inner(entries)?;
        Ok(())
    }
    pub fn write_anonymous_object(&mut self, entries: &HashMap<String, Value>) -> AmfWriteResult {
        let arr: Vec<(_, _)> = entries
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        self.write_anonymous_object_arr(arr.as_slice())?;
        Ok(())
    }
    fn write_null(&mut self) -> AmfWriteResult {
        self.inner.write_u8(amf0_marker::NULL)?;
        Ok(())
    }
    fn write_undefined(&mut self) -> AmfWriteResult {
        self.inner.write_u8(amf0_marker::UNDEFINED)?;
        Ok(())
    }
    fn write_reference(&mut self, index: u16) -> AmfWriteResult {
        self.inner.write_u8(amf0_marker::REFERENCE)?;
        self.inner.write_u16::<BigEndian>(index)?;
        Ok(())
    }
    fn write_ecma_array(&mut self, arr: &[(String, Value)]) -> AmfWriteResult {
        assert!(arr.len() <= 0xFFFF_FFFF);
        self.inner.write_u8(amf0_marker::ECMA_ARRAY)?;
        self.inner.write_u32::<BigEndian>(arr.len() as u32)?;
        self.write_pairs_inner(arr)?;
        Ok(())
    }
    fn write_object_end(&mut self) -> AmfWriteResult {
        self.inner.write_u8(amf0_marker::OBJECT_END)?;
        Ok(())
    }
    fn write_strict_array(&mut self, arr: &[Value]) -> AmfWriteResult {
        assert!(arr.len() <= 0xFFFF_FFFF);
        self.inner.write_u8(amf0_marker::STRICT_ARRAY)?;
        self.inner.write_u32::<BigEndian>(arr.len() as u32)?;
        for v in arr {
            self.write(v)?;
        }
        Ok(())
    }
    fn write_date(&mut self, date_time: time::Duration, time_zone: i16) -> AmfWriteResult {
        assert!(time_zone == 0x0000);
        self.inner.write_u8(amf0_marker::DATE)?;
        self.inner
            .write_f64::<BigEndian>(date_time.as_millis() as f64)?;
        self.inner.write_i16::<BigEndian>(0x0000)?;
        Ok(())
    }
    fn write_xml(&mut self, xml: &str) -> AmfWriteResult {
        self.inner.write_u8(amf0_marker::XML_DOCUMENT)?;
        self.write_long_string_inner(xml)?;
        Ok(())
    }
    fn write_typed_object_arr_inner(
        &mut self,
        name: &str,
        entries: &[(String, Value)],
    ) -> AmfWriteResult {
        assert!(entries.len() <= 0xFFFF_FFFF);
        self.inner.write_u8(amf0_marker::TYPED_OBJECT)?;
        self.write_short_string_inner(name)?;
        self.write_pairs_inner(entries)?;
        Ok(())
    }
    pub fn write_typed_object(
        &mut self,
        name: &str,
        entries: &HashMap<String, Value>,
    ) -> AmfWriteResult {
        let arr: Vec<(_, _)> = entries
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        self.write_typed_object_arr_inner(name, &arr)?;
        Ok(())
    }
    fn write_avm_plus(&mut self, value: &amf3::Value) -> AmfWriteResult {
        self.inner.write_u8(amf0_marker::AVMPLUS_OBJECT)?;
        amf3::Writer::new(&mut self.inner).write(value)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use core::time;

    use crate::{amf0::Value, amf3};

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
    fn number() {
        assert_eq!(
            encode!(Value::Number(3.5)),
            include_bytes!("../../test_data/amf0-number.bin")
        )
    }

    #[test]
    fn boolean() {
        assert_eq!(
            encode!(Value::Boolean(true)),
            include_bytes!("../../test_data/amf0-boolean-true.bin")
        );
        assert_eq!(
            encode!(Value::Boolean(false)),
            include_bytes!("../../test_data/amf0-boolean-false.bin")
        )
    }

    #[test]
    fn string() {
        assert_eq!(
            encode!(Value::String("this is a テスト".to_string())),
            include_bytes!("../../test_data/amf0-string.bin")
        );
    }

    #[test]
    fn anonymous_object() {
        {
            let arr = vec![
                ("utf".to_string(), Value::String("UTF テスト".to_string())),
                ("zed".to_string(), Value::Number(5.0)),
                (
                    "shift".to_string(),
                    Value::String("Shift テスト".to_string()),
                ),
            ];

            let mut buf = Vec::new();
            Writer::new(&mut buf)
                .write_anonymous_object_arr(&arr)
                .unwrap();

            assert_eq!(
                buf,
                include_bytes!("../../test_data/amf0-complex-encoded-string.bin")
            );
        }

        {
            let arr = vec![
                ("".to_string(), Value::String("".to_string())),
                ("foo".to_string(), Value::String("baz".to_string())),
                ("bar".to_string(), Value::Number(3.14)),
            ];

            let mut buf = Vec::new();
            Writer::new(&mut buf)
                .write_anonymous_object_arr(&arr)
                .unwrap();

            assert_eq!(buf, include_bytes!("../../test_data/amf0-object.bin"));
        }

        {
            let pairs = vec![
                ("foo".to_string(), Value::String("bar".to_string())),
                ("baz".to_string(), Value::Null),
            ];

            assert_eq!(
                encode!(Value::Object {
                    name: None,
                    entries: pairs
                }),
                include_bytes!("../../test_data/amf0-untyped-object.bin")
            );
        }
    }

    #[test]
    fn null() {
        assert_eq!(
            encode!(Value::Null),
            include_bytes!("../../test_data/amf0-null.bin")
        );
    }

    #[test]
    fn undefined() {
        assert_eq!(
            encode!(Value::Undefined),
            include_bytes!("../../test_data/amf0-undefined.bin")
        );
    }

    #[test]
    fn reference() {
        let pairs = vec![
            ("foo".to_string(), Value::String("baz".to_string())),
            ("bar".to_string(), Value::Number(3.14)),
        ];
        let object = Value::Object {
            name: None,
            entries: pairs,
        };
        let reference_pairs = vec![
            ("0".to_string(), object.clone()),
            ("1".to_string(), Value::Reference { index: 1 }),
        ];

        assert_eq!(
            encode!(Value::Object {
                name: None,
                entries: reference_pairs
            }),
            include_bytes!("../../test_data/amf0-ref-test.bin")
        )
    }

    #[test]
    fn ecma_array() {
        let arr = vec![
            ("0".to_string(), Value::String("a".to_string())),
            ("1".to_string(), Value::String("b".to_string())),
            ("2".to_string(), Value::String("c".to_string())),
            ("3".to_string(), Value::String("d".to_string())),
        ];
        assert_eq!(
            encode!(Value::ECMAArray(arr)),
            include_bytes!("../../test_data/amf0-ecma-ordinal-array.bin")
        );
    }

    #[test]
    fn strict_array() {
        let arr = vec![
            Value::Number(1.0),
            Value::String("2".to_string()),
            Value::Number(3.0),
        ];
        assert_eq!(
            encode!(Value::StrictArray(arr)),
            include_bytes!("../../test_data/amf0-strict-array.bin")
        );
    }

    #[test]
    fn date() {
        assert_eq!(
            encode!(Value::Date {
                time_zone: 0,
                millis_timestamp: time::Duration::from_millis(1_590_796_800_000)
            }),
            include_bytes!("../../test_data/amf0-date.bin")
        );

        assert_eq!(
            encode!(Value::Date {
                time_zone: 0,
                millis_timestamp: time::Duration::from_millis(1_045_112_400_000)
            }),
            include_bytes!("../../test_data/amf0-time.bin")
        );
    }

    #[test]
    fn xml() {
        assert_eq!(
            encode!(Value::XMLDocument(
                "<parent><child prop=\"test\" /></parent>".to_string()
            )),
            include_bytes!("../../test_data/amf0-xml-doc.bin")
        );
    }

    #[test]
    fn typed_object() {
        let arr = vec![
            ("foo".to_string(), Value::String("bar".to_string())),
            ("baz".to_string(), Value::Null),
        ];

        let mut buf = Vec::new();
        Writer::new(&mut buf)
            .write_typed_object_arr_inner("org.amf.ASClass", &arr)
            .unwrap();

        assert_eq!(buf, include_bytes!("../../test_data/amf0-typed-object.bin"));
    }

    #[test]
    fn avm_plus() {
        assert_eq!(
            encode!(Value::AVMPlus(amf3::Value::Array {
                assoc_entries: vec![],
                dense_entries: (1..4).map(amf3::Value::Integer).collect()
            })),
            include_bytes!("../../test_data/amf0-avmplus-object.bin")
        );
    }
}
