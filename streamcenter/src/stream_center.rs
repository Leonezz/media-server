use std::{backtrace::Backtrace, collections::HashMap};

use dashmap::DashMap;
use lazy_static::lazy_static;
use tokio::sync::{
    broadcast,
    mpsc::{self, Sender},
};
use tracing::instrument;

use crate::{
    errors::{StreamCenterError, StreamCenterResult},
    frame_info::FrameData,
    signal::StreamSignal,
    stream_source::{StreamIdentifier, StreamSource, StreamType},
};

#[derive(Debug)]
struct StreamSourceHandles {
    signal_sender: mpsc::Sender<StreamSignal>,
    subscribe_sender: broadcast::Sender<FrameData>,

    stream_identifier: StreamIdentifier,
    stream_type: StreamType,
}

#[derive(Debug)]
struct StreamCenter {
    streams: DashMap<StreamIdentifier, StreamSourceHandles>,
}

lazy_static! {
    static ref STREAM_CENTER: StreamCenter = StreamCenter {
        streams: DashMap::new(),
    };
}

#[instrument]
pub fn publish(
    stream_name: &str,
    app: &str,
    stream_type: StreamType,
    context: HashMap<String, serde_json::Value>,
) -> StreamCenterResult<Sender<FrameData>> {
    let stream_id = StreamIdentifier {
        app: app.to_string(),
        stream_name: stream_name.to_string(),
    };

    if STREAM_CENTER.streams.contains_key(&stream_id) {
        return Err(StreamCenterError::DuplicateStream(stream_id));
    }

    let (data_tx, data_rx) = mpsc::channel(128);
    let (signal_tx, signal_rx) = mpsc::channel(1);
    let (mut source, subscribe_sender) = StreamSource::new(
        stream_name,
        app,
        stream_type,
        context.clone(),
        data_rx,
        signal_rx,
    );

    tokio::spawn(async move { source.run().await });

    STREAM_CENTER
        .streams
        .insert(stream_id.clone().into(), StreamSourceHandles {
            signal_sender: signal_tx,
            subscribe_sender,

            stream_identifier: stream_id,
            stream_type,
        });

    tracing::info!(
        "publish new stream success, stream_name: {}, app: {}, stream_type: {}, context: {:?}. total stream count: {}",
        stream_name,
        app,
        stream_type,
        context,
        STREAM_CENTER.streams.len()
    );
    Ok(data_tx)
}

#[instrument]
pub async fn unpublish(stream_name: &str, app: &str) -> StreamCenterResult<()> {
    let stream_id = StreamIdentifier {
        stream_name: stream_name.to_string(),
        app: app.to_string(),
    };
    match STREAM_CENTER.streams.get_mut(&stream_id) {
        None => return Err(StreamCenterError::StreamNotFound(stream_id)),
        Some(handles) => handles
            .signal_sender
            .send(StreamSignal::Stop)
            .await
            .map_err(|err| StreamCenterError::ChannelSendFailed {
                backtrace: Backtrace::capture(),
            })?,
    };

    let removed = STREAM_CENTER.streams.remove(&stream_id);

    if removed.is_some() {
        let removed = removed.expect("this cannot be none");
        tracing::info!(
            "unpublish stream success, stream_name: {}, app: {}, stream_type: {}. total stream count: {}",
            removed.0.stream_name,
            removed.0.app,
            removed.1.stream_type,
            STREAM_CENTER.streams.len()
        );
    }

    Ok(())
}

#[instrument]
pub async fn subscribe(
    stream_name: &str,
    app: &str,
) -> StreamCenterResult<(broadcast::Receiver<FrameData>, StreamType)> {
    let stream_id = StreamIdentifier {
        stream_name: stream_name.to_string(),
        app: app.to_string(),
    };
    if !STREAM_CENTER.streams.contains_key(&stream_id) {
        return Err(StreamCenterError::StreamNotFound(stream_id));
    }

    match STREAM_CENTER.streams.get(&stream_id) {
        None => return Err(StreamCenterError::StreamNotFound(stream_id)),
        Some(handles) => return Ok((handles.subscribe_sender.subscribe(), handles.stream_type)),
    }
}
