#[cfg(test)]
mod tests {
    use core::time;

    use crate::amf3::{self, Value, Writer};

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
        assert_eq!(
            amf3::Reader::new(&mut &buf[..]).read().unwrap().unwrap(),
            value
        );
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

        assert_eq!(
            amf3::Reader::new(&mut &buf[..]).read().unwrap().unwrap(),
            value
        );
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
        assert_eq!(
            amf3::Reader::new(&mut &buf[..]).read().unwrap().unwrap(),
            value
        );

        assert_eq!(
            encode!(Value::Dictionary {
                is_weak: false,
                entries: vec![]
            }),
            include_bytes!("../../test_data/amf3-empty-dictionary.bin")
        );
    }
}
