use std::{collections::HashMap, fmt::Display};

use tokio::sync::{
    broadcast::{self},
    mpsc,
};

use crate::{
    errors::{StreamCenterError, StreamCenterResult},
    frame_info::FrameData,
    gop::Gop,
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
    data_distributer: broadcast::Sender<FrameData>,
    // data_consumer: broadcast::Receiver<FrameData>,
    status: StreamStatus,
    signal_receiver: mpsc::Receiver<StreamSignal>,
    gop_cache: Vec<Gop>,
}

impl StreamSource {
    pub fn new(
        stream_name: &str,
        app: &str,
        stream_type: StreamType,
        context: HashMap<String, serde_json::Value>,
        data_receiver: mpsc::Receiver<FrameData>,
        signal_receiver: mpsc::Receiver<StreamSignal>,
    ) -> (Self, broadcast::Sender<FrameData>) {
        let (tx, _) = broadcast::channel(128);
        (
            Self {
                identifier: StreamIdentifier {
                    stream_name: stream_name.to_string(),
                    app: app.to_string(),
                },
                stream_type,
                context,
                data_receiver,
                data_distributer: tx.clone(),
                // data_consumer: rx,
                gop_cache: Vec::new(),
                status: StreamStatus::NotStarted,
                signal_receiver,
            },
            tx,
        )
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
                    self.on_frame_data(data)?;
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

    fn on_frame_data(&mut self, data: FrameData) -> StreamCenterResult<()> {
        Ok(())
    }
}
