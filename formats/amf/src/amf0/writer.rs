use core::time;
use std::{collections::HashMap, io};

use crate::{
    amf3,
    errors::{AmfError, AmfResult},
};

use byteorder::{BigEndian, WriteBytesExt};
use utils::traits::writer::WriteTo;

use super::{Value, amf0_marker};

impl<W: io::Write> WriteTo<W> for Value {
    type Error = AmfError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        match self {
            Value::Number(n) => Self::write_number(writer, *n),
            Value::Boolean(b) => Self::write_boolean(writer, *b),
            Value::String(ss) => Self::write_string(writer, ss),
            Value::Object { name, entries } => match name {
                Some(name) => Self::write_typed_object_arr_inner(writer, name, entries),
                None => Self::write_anonymous_object_arr(writer, entries),
            },
            Value::Null => Self::write_null(writer),
            Value::Undefined => Self::write_undefined(writer),
            Value::Reference { index } => Self::write_reference(writer, *index),
            Value::ECMAArray(arr) => Self::write_ecma_array(writer, arr),
            Value::ObjectEnd => Self::write_object_end(writer),
            Value::StrictArray(arr) => Self::write_strict_array(writer, arr),
            Value::Date {
                time_zone,
                millis_timestamp: unix_timestamp,
            } => Self::write_date(writer, unix_timestamp, *time_zone),
            Value::XMLDocument(xml) => Self::write_xml(writer, xml),
            Value::AVMPlus(value) => Self::write_avm_plus(writer, value),
        }
    }
}

impl Value {
    pub fn write_number<W: io::Write>(writer: &mut W, v: f64) -> AmfResult<()> {
        writer.write_u8(amf0_marker::NUMBER)?;
        writer.write_f64::<BigEndian>(v)?;
        Ok(())
    }
    pub fn write_boolean<W: io::Write>(writer: &mut W, v: bool) -> AmfResult<()> {
        writer.write_u8(amf0_marker::BOOLEAN)?;
        writer.write_u8(v as u8)?;
        Ok(())
    }
    fn write_short_string_inner<W: io::Write>(writer: &mut W, v: &str) -> AmfResult<()> {
        assert!(v.len() < 0xFFFF); // TODO CHECK this
        writer.write_u16::<BigEndian>(v.len() as u16)?;
        writer.write_all(v.as_bytes())?;
        Ok(())
    }
    fn write_long_string_inner<W: io::Write>(writer: &mut W, v: &str) -> AmfResult<()> {
        assert!(v.len() <= 0xFFFF_FFFF);
        writer.write_u32::<BigEndian>(v.len() as u32)?;
        writer.write_all(v.as_bytes())?;
        Ok(())
    }
    pub fn write_string<W: io::Write>(writer: &mut W, v: &str) -> AmfResult<()> {
        if v.len() < 0xFFFF {
            writer.write_u8(amf0_marker::STRING)?;
            Self::write_short_string_inner(writer, v)?;
        } else {
            writer.write_u8(amf0_marker::LONG_STRING)?;
            Self::write_long_string_inner(writer, v)?;
        }
        Ok(())
    }
    fn write_pairs_inner<W: io::Write>(
        writer: &mut W,
        entries: &[(String, Value)],
    ) -> AmfResult<()> {
        for (key, value) in entries {
            Self::write_short_string_inner(writer, key)?;
            value.write_to(writer)?;
        }
        writer.write_u16::<BigEndian>(0)?;
        writer.write_u8(amf0_marker::OBJECT_END)?;
        Ok(())
    }
    fn write_anonymous_object_arr<W: io::Write>(
        writer: &mut W,
        entries: &[(String, Value)],
    ) -> AmfResult<()> {
        assert!(entries.len() <= 0xFFFF_FFFF);
        writer.write_u8(amf0_marker::OBJECT)?;
        Self::write_pairs_inner(writer, entries)?;
        Ok(())
    }
    pub fn write_anonymous_object<W: io::Write>(
        writer: &mut W,
        entries: &HashMap<String, Value>,
    ) -> AmfResult<()> {
        let arr: Vec<(_, _)> = entries
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Self::write_anonymous_object_arr(writer, arr.as_slice())?;
        Ok(())
    }
    pub fn write_null<W: io::Write>(writer: &mut W) -> AmfResult<()> {
        writer.write_u8(amf0_marker::NULL)?;
        Ok(())
    }
    pub fn write_undefined<W: io::Write>(writer: &mut W) -> AmfResult<()> {
        writer.write_u8(amf0_marker::UNDEFINED)?;
        Ok(())
    }
    pub fn write_reference<W: io::Write>(writer: &mut W, index: u16) -> AmfResult<()> {
        writer.write_u8(amf0_marker::REFERENCE)?;
        writer.write_u16::<BigEndian>(index)?;
        Ok(())
    }
    pub fn write_ecma_array<W: io::Write>(
        writer: &mut W,
        arr: &[(String, Value)],
    ) -> AmfResult<()> {
        assert!(arr.len() <= 0xFFFF_FFFF);
        writer.write_u8(amf0_marker::ECMA_ARRAY)?;
        writer.write_u32::<BigEndian>(arr.len() as u32)?;
        Self::write_pairs_inner(writer, arr)?;
        Ok(())
    }
    fn write_object_end<W: io::Write>(writer: &mut W) -> AmfResult<()> {
        writer.write_u8(amf0_marker::OBJECT_END)?;
        Ok(())
    }
    pub fn write_strict_array<W: io::Write>(writer: &mut W, arr: &[Value]) -> AmfResult<()> {
        assert!(arr.len() <= 0xFFFF_FFFF);
        writer.write_u8(amf0_marker::STRICT_ARRAY)?;
        writer.write_u32::<BigEndian>(arr.len() as u32)?;
        for v in arr {
            v.write_to(writer)?;
        }
        Ok(())
    }
    pub fn write_date<W: io::Write>(
        writer: &mut W,
        date_time: &time::Duration,
        time_zone: i16,
    ) -> AmfResult<()> {
        assert!(time_zone.eq(&0x0000));
        writer.write_u8(amf0_marker::DATE)?;
        writer.write_f64::<BigEndian>(date_time.as_millis() as f64)?;
        writer.write_i16::<BigEndian>(0x0000)?;
        Ok(())
    }
    pub fn write_xml<W: io::Write>(writer: &mut W, xml: &str) -> AmfResult<()> {
        writer.write_u8(amf0_marker::XML_DOCUMENT)?;
        Self::write_long_string_inner(writer, xml)?;
        Ok(())
    }
    fn write_typed_object_arr_inner<W: io::Write>(
        writer: &mut W,
        name: &str,
        entries: &[(String, Value)],
    ) -> AmfResult<()> {
        assert!(entries.len() <= 0xFFFF_FFFF);
        writer.write_u8(amf0_marker::TYPED_OBJECT)?;
        Self::write_short_string_inner(writer, name)?;
        Self::write_pairs_inner(writer, entries)?;
        Ok(())
    }
    pub fn write_typed_object<W: io::Write>(
        writer: &mut W,
        name: &str,
        entries: &HashMap<String, Value>,
    ) -> AmfResult<()> {
        let arr: Vec<(_, _)> = entries
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Self::write_typed_object_arr_inner(writer, name, &arr)?;
        Ok(())
    }
    pub fn write_avm_plus<W: io::Write>(writer: &mut W, value: &amf3::Value) -> AmfResult<()> {
        writer.write_u8(amf0_marker::AVMPLUS_OBJECT)?;
        value.write_to(writer)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use core::time;

    use crate::{amf0::Value, amf3};
    use utils::traits::writer::WriteTo;

    macro_rules! encode {
        ($value:expr) => {{
            let mut buf = Vec::new();
            let res = (&$value).write_to(&mut buf);
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
            Value::write_anonymous_object_arr(&mut buf, &arr).unwrap();

            assert_eq!(
                buf,
                include_bytes!("../../test_data/amf0-complex-encoded-string.bin")
            );
        }

        {
            let arr = vec![
                ("".to_string(), Value::String("".to_string())),
                ("foo".to_string(), Value::String("baz".to_string())),
                #[allow(clippy::approx_constant)]
                ("bar".to_string(), Value::Number(3.14)),
            ];

            let mut buf = Vec::new();
            Value::write_anonymous_object_arr(&mut buf, &arr).unwrap();

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
            #[allow(clippy::approx_constant)]
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
        Value::write_typed_object_arr_inner(&mut buf, "org.amf.ASClass", &arr).unwrap();

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
