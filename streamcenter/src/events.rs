use std::collections::HashMap;

use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::{
    errors::StreamCenterResult,
    frame_info::ChunkFrameData,
    gop::FLVMediaFrame,
    stream_source::{StreamIdentifier, StreamType},
};

#[derive(Debug)]
pub enum StreamCenterEvent {
    Publish {
        stream_type: StreamType,
        stream_id: StreamIdentifier,
        context: HashMap<String, serde_json::Value>,
        result_sender: oneshot::Sender<StreamCenterResult<mpsc::Sender<ChunkFrameData>>>, // success or not
    },
    Unpublish {
        stream_id: StreamIdentifier,
        result_sender: oneshot::Sender<StreamCenterResult<()>>,
    },
    Subscribe {
        stream_id: StreamIdentifier,
        result_sender: oneshot::Sender<StreamCenterResult<SubscribeResponse>>,
    },
    Unsubscribe {
        stream_id: StreamIdentifier,
        uuid: Uuid,
        result_sender: oneshot::Sender<StreamCenterResult<()>>,
    },
}

#[derive(Debug)]
pub struct SubscribeResponse {
    pub subscribe_id: Uuid,
    pub stream_type: StreamType,
    pub has_video: bool,
    pub has_audio: bool,
    pub media_receiver: mpsc::Receiver<FLVMediaFrame>,
}
