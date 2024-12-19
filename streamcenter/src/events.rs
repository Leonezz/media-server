use std::collections::HashMap;

use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::{
    errors::StreamCenterResult,
    frame_info::FrameData,
    stream_source::{StreamIdentifier, StreamType},
};

#[derive(Debug)]
pub enum StreamCenterEvent {
    Publish {
        stream_type: StreamType,
        stream_id: StreamIdentifier,
        context: HashMap<String, serde_json::Value>,
        result_sender: oneshot::Sender<StreamCenterResult<mpsc::Sender<FrameData>>>, // success or not
    },
    Unpublish {
        stream_id: StreamIdentifier,
        result_sender: oneshot::Sender<StreamCenterResult<()>>,
    },
    Subscribe {
        stream_id: StreamIdentifier,
        result_sender:
            oneshot::Sender<StreamCenterResult<(Uuid, StreamType, mpsc::Receiver<FrameData>)>>,
    },
    Unsubscribe {
        stream_id: StreamIdentifier,
        uuid: Uuid,
        result_sender: oneshot::Sender<StreamCenterResult<()>>,
    },
}
