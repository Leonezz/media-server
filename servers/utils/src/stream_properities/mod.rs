use std::collections::HashMap;

use errors::StreamPropertiesError;
use url::Url;

pub mod errors;

#[derive(Debug, Default)]
pub struct StreamProperties {
    pub stream_name: String,
    pub app: String,
    pub stream_context: HashMap<String, String>,
}

impl TryFrom<&Url> for StreamProperties {
    type Error = StreamPropertiesError;
    fn try_from(value: &Url) -> Result<Self, Self::Error> {
        let path_segs = value.path_segments();
        if path_segs.is_none() {
            return Err(StreamPropertiesError::ParseFromUrlFailed(format!(
                "invalid url: {}",
                value
            )));
        }

        let path_segs = path_segs.unwrap();
        let path_seg_vec: Vec<_> = path_segs.collect();
        if path_seg_vec.len() < 2 {
            return Err(StreamPropertiesError::ParseFromUrlFailed(format!(
                "invalid url: {}",
                value
            )));
        }
        let app = path_seg_vec[0].to_owned();
        let stream_name = path_seg_vec[1].to_owned();
        let mut context = HashMap::new();
        for (k, v) in value.query_pairs() {
            context.insert(k.to_string(), v.to_string());
        }
        Ok(Self {
            stream_name,
            app,
            stream_context: context,
        })
    }
}
