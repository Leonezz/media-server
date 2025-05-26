use std::collections::HashMap;

use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::{
    errors::StreamCenterResult,
    gop::MediaFrame,
    stream_source::{PlayProtocol, PublishProtocol, StreamIdentifier, StreamType},
};

#[derive(Debug)]
pub enum StreamCenterEvent {
    Publish {
        stream_type: StreamType,
        protocol: PublishProtocol,
        stream_id: StreamIdentifier,
        context: HashMap<String, String>,
        result_sender: oneshot::Sender<StreamCenterResult<mpsc::Sender<MediaFrame>>>, // success or not
    },
    Unpublish {
        stream_id: StreamIdentifier,
        result_sender: oneshot::Sender<StreamCenterResult<()>>,
    },
    Subscribe {
        stream_id: StreamIdentifier,
        protocol: PlayProtocol,
        context: HashMap<String, String>,
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
    pub media_receiver: mpsc::Receiver<MediaFrame>,
}
