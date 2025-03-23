use std::{
    collections::HashMap,
    io::{Cursor, Read},
};

use tokio_util::bytes::BytesMut;

use super::OnMetaData;

impl OnMetaData {
    pub fn read_from(
        reader: &mut Cursor<&BytesMut>,
        amf_version: amf_formats::Version,
    ) -> Option<OnMetaData> {
        let name = amf_formats::Value::read_from(reader.by_ref(), amf_version).unwrap_or(None);
        let name_valid = match name {
            None => false,
            Some(name) => match name.try_as_str() {
                None => false,
                Some(name_str) => name_str == "@setDataFrame",
            },
        };
        if !name_valid {
            return None;
        }
        let name = amf_formats::Value::read_from(reader.by_ref(), amf_version).unwrap_or(None);
        let name_valid = match name {
            None => false,
            Some(name) => match name.try_as_str() {
                None => false,
                Some(name_str) => name_str == "onMetaData",
            },
        };

        if !name_valid {
            return None;
        }

        let key_value_pairs = amf_formats::Value::read_from(reader, amf_version).unwrap_or(None);
        match key_value_pairs {
            None => None,
            Some(value) => match value.try_into_pairs() {
                Err(_err) => None,
                Ok(pairs) => {
                    let mut value_map: HashMap<String, amf_formats::Value> = HashMap::new();
                    for (k, v) in pairs {
                        value_map.insert(k, v);
                    }
                    Some(value_map.into())
                }
            },
        }
    }
}
