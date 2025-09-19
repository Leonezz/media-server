use crate::{
    errors::StreamCenterResult,
    gop::MediaFrame,
    stream_source::{
        ParsedContext, PlayProtocol, PlayStat, PublishProtocol, StreamIdentifier, SubscribeHandler,
    },
};
use codec_common::{audio::AudioConfig, video::VideoConfig};
use std::{collections::HashMap, time::SystemTime};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

#[derive(Debug)]
pub enum StreamCenterEvent {
    Publish {
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
    Describe {
        stream_id: StreamIdentifier,
        result_sender: oneshot::Sender<StreamCenterResult<StreamDescription>>,
    },
}

#[derive(Debug)]
pub struct SubscriberInfo {
    pub id: Uuid,
    pub play_protocol: PlayProtocol,
    pub context: HashMap<String, String>,
    pub parsed_context: ParsedContext,
    pub play_stat: PlayStat,
}

impl From<&SubscribeHandler> for SubscriberInfo {
    fn from(value: &SubscribeHandler) -> Self {
        Self {
            id: value.id,
            play_protocol: value.play_protocol,
            context: value.context.clone(),
            parsed_context: value.parsed_context.clone(),
            play_stat: value.stat.clone(),
        }
    }
}

#[derive(Debug)]
pub struct StreamDescription {
    pub publish_protocol: PublishProtocol,
    pub stream_id: StreamIdentifier,
    pub video_config: Option<VideoConfig>,
    pub has_video: bool,
    pub audio_conifg: Option<AudioConfig>,
    pub has_audio: bool,
    pub publish_start_time: SystemTime,
    pub subscribers: HashMap<Uuid, SubscriberInfo>,
}

#[derive(Debug)]
pub struct SubscribeResponse {
    pub subscribe_id: Uuid,
    pub has_video: bool,
    pub has_audio: bool,
    pub media_receiver: mpsc::Receiver<MediaFrame>,
}
