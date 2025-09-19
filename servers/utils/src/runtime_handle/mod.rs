use std::{sync::Arc, time::SystemTime};

use stream_center::gop::MediaFrame;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug)]
pub struct PlayHandle {
    pub stream_data_consumer: tokio::sync::mpsc::Receiver<MediaFrame>,
    pub play_id: Uuid,
    pub receive_audio: bool,
    pub receive_video: bool,
    pub buffer_length: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct PublishHandle {
    pub stream_data_producer: tokio::sync::mpsc::Sender<MediaFrame>,
    pub no_data_since: Option<SystemTime>,
}

#[derive(Debug)]
pub enum SessionRuntime {
    Play(Arc<RwLock<PlayHandle>>),
    Publish(Arc<RwLock<PublishHandle>>),
    Unknown,
}

impl SessionRuntime {
    pub fn is_publish(&self) -> bool {
        matches!(self, Self::Publish(_))
    }

    pub fn is_play(&self) -> bool {
        matches!(self, Self::Play(_))
    }

    pub fn get_publish_handle(&self) -> Option<&Arc<RwLock<PublishHandle>>> {
        match self {
            Self::Publish(h) => Some(h),
            _ => None,
        }
    }

    pub fn get_play_handle(&self) -> Option<&Arc<RwLock<PlayHandle>>> {
        match self {
            Self::Play(p) => Some(p),
            _ => None,
        }
    }
}
