use std::{any, collections::HashMap};

use dashmap::DashMap;
use lazy_static::lazy_static;
use tokio::sync::mpsc::{self, Sender};

use crate::{
    errors::{StreamCenterError, StreamCenterResult},
    frame_info::FrameData,
    stream_source::StreamSource,
};

#[derive(Debug)]
struct StreamCenter {
    streams: DashMap<String, StreamSource>,
}

lazy_static! {
    static ref STREAM_CENTER: StreamCenter = StreamCenter {
        streams: DashMap::new()
    };
}

pub fn publish(
    stream_name: &str,
    app: &str,
    context: HashMap<String, serde_json::Value>,
) -> StreamCenterResult<Sender<FrameData>> {
    let stream_center = &STREAM_CENTER;
    if stream_center.streams.contains_key(stream_name) {
        return Err(StreamCenterError::DuplicateStream(stream_name.into()));
    }

    let (tx, rx) = mpsc::channel(128);
    let source: StreamSource = StreamSource::new(stream_name, app, context, rx);
    stream_center.streams.insert(stream_name.into(), source);
    Ok(tx)
}
