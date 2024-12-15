use std::collections::HashMap;

use tokio::sync::{
    broadcast::{self},
    mpsc,
};

use crate::{
    errors::StreamCenterResult, frame_info::FrameData, gop::Gop, signal::StreamSignal,
    util::concat_stream_id,
};

#[derive(Debug, PartialEq, Eq)]
enum StreamStatus {
    NotStarted,
    Running,
    Stopped,
}

#[derive(Debug)]
pub struct StreamSource {
    stream_name: String,
    app: String,
    stream_type: String,
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
        stream_type: &str,
        context: HashMap<String, serde_json::Value>,
        data_receiver: mpsc::Receiver<FrameData>,
        signal_receiver: mpsc::Receiver<StreamSignal>,
    ) -> (Self, broadcast::Sender<FrameData>) {
        let (tx, _) = broadcast::channel(128);
        (
            Self {
                stream_name: stream_name.into(),
                app: app.into(),
                stream_type: stream_type.into(),
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
        let stream_id = concat_stream_id(&self.stream_name, &self.app);
        tracing::info!("stream is running, stream id: {}", stream_id);

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
