use std::{collections::HashMap, io};

use utils::traits::reader::ReadRemainingFrom;

use crate::errors::FLVError;

use super::OnMetaData;

impl<R: io::Read> ReadRemainingFrom<amf_formats::Version, R> for OnMetaData {
    type Error = FLVError;
    fn read_remaining_from(
        header: amf_formats::Version,
        mut reader: R,
    ) -> Result<Self, Self::Error> {
        let name = amf_formats::Value::read_remaining_from(header, reader.by_ref())?;
        let name_valid = match name.try_as_str() {
            None => false,
            Some(name) => name == "@setDataFrame",
        };
        if !name_valid {
            return Err(FLVError::InvalidOnMetaData(format!(
                "expect @setDataFrame, got: {:?}",
                name
            )));
        }
        let name = amf_formats::Value::read_remaining_from(header, reader.by_ref())?;
        let name_valid = match name.try_as_str() {
            None => false,
            Some(name) => name == "onMetaData",
        };

        if !name_valid {
            return Err(FLVError::InvalidOnMetaData(format!(
                "expect onMetaData, got: {:?}",
                name
            )));
        }

        let key_value_pairs =
            amf_formats::Value::read_remaining_from(header, reader)?.try_into_pairs();

        match key_value_pairs {
            Err(value) => Err(FLVError::InvalidOnMetaData(format!(
                "expect key value pairs, got {:?}",
                value
            ))),
            Ok(pairs) => {
                let mut value_map: HashMap<String, amf_formats::Value> = HashMap::new();
                for (k, v) in pairs {
                    value_map.insert(k, v);
                }
                Ok(value_map.into())
            }
        }
    }
}
