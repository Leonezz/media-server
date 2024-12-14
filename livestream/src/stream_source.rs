use std::{any, collections::HashMap};

use tokio::sync::{broadcast, mpsc};

use crate::{frame_info::FrameData, gop::Gop};

#[derive(Debug)]
pub struct StreamSource {
    stream_name: String,
    app: String,
    context: HashMap<String, serde_json::Value>,
    data_receiver: mpsc::Receiver<FrameData>,
    data_distributer: broadcast::Sender<FrameData>,
    // data_consumer: broadcast::Receiver<FrameData>,
    gop_cache: Vec<Gop>,
}

impl StreamSource {
    pub fn new(
        stream_name: &str,
        app: &str,
        context: HashMap<String, serde_json::Value>,
        data_receiver: mpsc::Receiver<FrameData>,
    ) -> Self {
        let (tx, _) = broadcast::channel(128);
        Self {
            stream_name: stream_name.into(),
            app: app.into(),
            context,
            data_receiver,
            data_distributer: tx,
            // data_consumer: rx,
            gop_cache: Vec::new(),
        }
    }

    pub fn get_consumer(&mut self) -> broadcast::Receiver<FrameData> {
        self.data_distributer.subscribe()
    }
}
