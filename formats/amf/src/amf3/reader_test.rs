#[cfg(test)]
mod tests {
    use core::{f64, time};
    use std::{
        io::{self},
        vec,
    };

    use crate::{amf3::Reader, amf3::Value, errors::AmfError};

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
            AmfError::InvalidDate {
                milliseconds: f64::INFINITY
            }
        ));
        assert!(matches!(
            decode!("../../test_data/amf3-date-minus-millis.bin").unwrap_err(),
            AmfError::InvalidDate { milliseconds: -1.0 }
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
            AmfError::UnsupportedExternalizable {
                name
            } if name == *"ExternalizableTest"
        ));

        assert!(matches!(
            decode!("../../test_data/amf3-array-collection.bin").unwrap_err(),
            AmfError::UnsupportedExternalizable { name } if name == *"flex.messaging.io.ArrayCollection"
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
            AmfError::CircularReference { index: 0 }
        ));

        assert!(matches!(
            decode!("../../test_data/amf3-bad-object-ref.bin").unwrap_err(),
            AmfError::OutOfRangeReference { index: 10 }
        ));

        assert!(matches!(
            decode!("../../test_data/amf3-bad-trait-ref.bin").unwrap_err(),
            AmfError::OutOfRangeReference { index: 4 }
        ));

        assert!(matches!(
            decode!("../../test_data/amf3-bad-string-ref.bin").unwrap_err(),
            AmfError::OutOfRangeReference { index: 8 }
        ));
    }

    #[test]
    fn unknown_marker() {
        assert!(matches!(
            decode!("../../test_data/amf3-unknown-marker.bin").unwrap_err(),
            AmfError::Unknown { marker: 123 }
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
