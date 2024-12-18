use core::time;
use std::{
    backtrace::Backtrace,
    collections::{HashMap, VecDeque},
    fmt::Display,
    sync::Arc,
};

use tokio::sync::{
    RwLock,
    broadcast::{self},
    mpsc,
};
use uuid::Uuid;

use crate::{
    errors::{StreamCenterError, StreamCenterResult},
    frame_info::FrameData,
    gop::{Gop, GopCache},
    signal::StreamSignal,
};

#[derive(Debug, PartialEq, Eq)]
enum StreamStatus {
    NotStarted,
    Running,
    Stopped,
}

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq, Default)]
pub enum StreamType {
    #[default]
    Live,
    Record,
    Append,
}

impl Display for StreamType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Live => f.write_str("live"),
            Self::Record => f.write_str("record"),
            Self::Append => f.write_str("append"),
        }
    }
}

impl Into<String> for StreamType {
    fn into(self) -> String {
        match self {
            Self::Live => "live".to_string(),
            Self::Record => "record".to_string(),
            Self::Append => "append".to_string(),
        }
    }
}

impl TryFrom<String> for StreamType {
    type Error = StreamCenterError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "live" => Ok(StreamType::Live),
            "recorded" => Ok(StreamType::Record),
            "append" => Ok(StreamType::Append),
            _ => Err(StreamCenterError::InvalidStreamType(value.into())),
        }
    }
}

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct StreamIdentifier {
    pub stream_name: String,
    pub app: String,
}

#[derive(Debug)]
pub struct StreamSource {
    pub identifier: StreamIdentifier,
    pub stream_type: StreamType,

    context: HashMap<String, serde_json::Value>,
    data_receiver: mpsc::Receiver<FrameData>,
    data_distributer: Arc<RwLock<HashMap<Uuid, mpsc::Sender<FrameData>>>>,
    // data_consumer: broadcast::Receiver<FrameData>,
    status: StreamStatus,
    signal_receiver: mpsc::Receiver<StreamSignal>,
    gop_cache: GopCache,
}

impl StreamSource {
    pub fn new(
        stream_name: &str,
        app: &str,
        stream_type: StreamType,
        context: HashMap<String, serde_json::Value>,
        data_receiver: mpsc::Receiver<FrameData>,
        signal_receiver: mpsc::Receiver<StreamSignal>,
        data_distributer: Arc<RwLock<HashMap<Uuid, mpsc::Sender<FrameData>>>>,
    ) -> Self {
        Self {
            identifier: StreamIdentifier {
                stream_name: stream_name.to_string(),
                app: app.to_string(),
            },
            stream_type,
            context,
            data_receiver,
            data_distributer,
            // data_consumer: rx,
            gop_cache: GopCache::new(100_1000, 1000_1000_1000),
            status: StreamStatus::NotStarted,
            signal_receiver,
        }
    }

    pub async fn run(&mut self) -> StreamCenterResult<()> {
        if self.status == StreamStatus::Running {
            return Ok(());
        }
        self.status = StreamStatus::Running;
        tracing::info!("stream is running, stream id: {:?}", self.identifier);

        loop {
            match self.data_receiver.recv().await {
                None => {}
                Some(data) => {
                    self.on_frame_data(data).await?;
                    tracing::info!("get frame");
                }
            }
            match self.signal_receiver.try_recv() {
                Err(_) => {}
                Ok(signal) => match signal {
                    StreamSignal::Stop => {
                        self.status = StreamStatus::Stopped;
                        return Ok(());
                    }
                },
            }
        }
    }

    async fn on_frame_data(&mut self, data: FrameData) -> StreamCenterResult<()> {
        self.gop_cache.append_frame(data.clone());
        if self.data_distributer.read().await.len() == 0 {
            return Ok(());
        }
        tracing::info!(
            "stream source consumer count: {}",
            self.data_distributer.read().await.len()
        );
        for (key, sender) in &mut self.data_distributer.read().await.iter() {
            let res = sender
                .send_timeout(data.clone(), time::Duration::from_millis(100))
                .await;
            if res.is_err() {
                tracing::error!("distribute frame data to {} failed: {:?}", key, res);
            }
        }

        Ok(())
    }
}
