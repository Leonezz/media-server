use std::{backtrace::Backtrace, collections::HashMap, sync::Arc};

use dashmap::DashMap;
use lazy_static::lazy_static;
use tokio::sync::{
    RwLock, broadcast,
    mpsc::{self, Sender},
};
use tracing::instrument;
use uuid::Uuid;

use crate::{
    errors::{StreamCenterError, StreamCenterResult},
    events::StreamCenterEvent,
    frame_info::FrameData,
    signal::StreamSignal,
    stream_source::{StreamIdentifier, StreamSource, StreamType},
};

#[derive(Debug)]
struct StreamSourceHandles {
    signal_sender: mpsc::Sender<StreamSignal>,
    source_sender: mpsc::Sender<FrameData>,

    stream_identifier: StreamIdentifier,
    stream_type: StreamType,

    data_distributer: Arc<RwLock<HashMap<Uuid, mpsc::Sender<FrameData>>>>,
}

#[derive(Debug)]
pub struct StreamCenter {
    streams: HashMap<StreamIdentifier, StreamSourceHandles>,
    event_receiver: mpsc::UnboundedReceiver<StreamCenterEvent>,
    event_sender: mpsc::UnboundedSender<StreamCenterEvent>,
}

impl StreamCenter {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            streams: HashMap::new(),
            event_receiver: rx,
            event_sender: tx,
        }
    }

    pub fn get_event_sender(&self) -> mpsc::UnboundedSender<StreamCenterEvent> {
        self.event_sender.clone()
    }

    pub async fn run(&mut self) -> StreamCenterResult<()> {
        loop {
            match self.event_receiver.recv().await {
                None => {}
                Some(event) => {
                    if let Err(err) = self.process_event(event).await {
                        tracing::error!("process stream center event failed, {:?}", err);
                    }
                }
            }
        }
    }

    async fn process_event(&mut self, event: StreamCenterEvent) -> StreamCenterResult<()> {
        tracing::info!("process event: {:?}", event);
        match event {
            StreamCenterEvent::Publish {
                stream_type,
                stream_id,
                context,
                result_sender,
            } => {
                self.process_publish_event(stream_type, stream_id, context, result_sender)
                    .await?
            }
            StreamCenterEvent::Unpublish {
                stream_id,
                result_sender,
            } => {
                self.process_unpublish_event(stream_id, result_sender)
                    .await?
            }
            StreamCenterEvent::Subscribe {
                stream_id,
                result_sender,
            } => {
                self.process_subscribe_event(stream_id, result_sender)
                    .await?
            }
            StreamCenterEvent::Unsubscribe {
                stream_id,
                uuid,
                result_sender,
            } => {
                self.process_unsubscribe_event(uuid, stream_id, result_sender)
                    .await?
            }
        }
        Ok(())
    }

    async fn process_publish_event(
        &mut self,
        stream_type: StreamType,
        stream_id: StreamIdentifier,
        context: HashMap<String, serde_json::Value>,
        result_sender: Sender<StreamCenterResult<mpsc::Sender<FrameData>>>,
    ) -> StreamCenterResult<()> {
        if self.streams.contains_key(&stream_id) {
            return result_sender
                .send(Err(StreamCenterError::DuplicateStream(stream_id.clone())))
                .await
                .map_err(|err| {
                    tracing::error!("deliver publish fail result to caller failed, {:?}", err);
                    return StreamCenterError::ChannelSendFailed {
                        backtrace: Backtrace::capture(),
                    };
                });
        }

        let (frame_sender, frame_receiver) = mpsc::channel(128);
        let (signal_sender, signal_receiver) = mpsc::channel(1);
        let data_distributer = Arc::new(RwLock::new(HashMap::new()));
        let mut source = StreamSource::new(
            &stream_id.stream_name,
            &stream_id.app,
            stream_type,
            context.clone(),
            frame_receiver,
            signal_receiver,
            data_distributer.clone(),
        );

        tokio::spawn(async move { source.run().await });

        self.streams.insert(stream_id.clone(), StreamSourceHandles {
            signal_sender,
            source_sender: frame_sender.clone(),
            stream_identifier: stream_id.clone(),
            stream_type,
            data_distributer,
        });

        result_sender.send(Ok(frame_sender)).await.map_err(|err| {
            tracing::error!("deliver publish success result to caller failed, {:?}", err);
            return StreamCenterError::ChannelSendFailed {
                backtrace: Backtrace::capture(),
            };
        })?;

        tracing::info!(
            "publish new stream success, stream_name: {}, app: {}, stream_type: {}, context: {:?}. total stream count: {}",
            &stream_id.stream_name,
            &stream_id.app,
            stream_type,
            context,
            self.streams.len()
        );

        Ok(())
    }

    async fn process_unpublish_event(
        &mut self,
        stream_id: StreamIdentifier,
        result_sender: Sender<StreamCenterResult<()>>,
    ) -> StreamCenterResult<()> {
        match self.streams.get_mut(&stream_id) {
            None => result_sender
                .send(Err(StreamCenterError::StreamNotFound(stream_id.clone())))
                .await
                .map_err(|err| {
                    tracing::error!(
                        "deliver unpublish failed result to caller failed, {:?}",
                        err
                    );
                    return StreamCenterError::ChannelSendFailed {
                        backtrace: Backtrace::capture(),
                    };
                }),
            Some(handles) => handles
                .signal_sender
                .send(StreamSignal::Stop)
                .await
                .map_err(|err| {
                    tracing::error!("send stop signal to stream source failed, {:?}", err);
                    return StreamCenterError::ChannelSendFailed {
                        backtrace: Backtrace::capture(),
                    };
                }),
        }?;

        let removed = self.streams.remove(&stream_id);

        if removed.is_some() {
            let removed = removed.expect("this cannot be none");
            tracing::info!(
                "unpublish stream success, stream_name: {}, app: {}, stream_type: {}. total stream count: {}",
                removed.stream_identifier.stream_name,
                removed.stream_identifier.app,
                removed.stream_type,
                self.streams.len()
            );
        }

        result_sender.send(Ok(())).await.map_err(|err| {
            tracing::error!(
                "deliver unpublish success result to caller failed, {:?}",
                err
            );
            return StreamCenterError::ChannelSendFailed {
                backtrace: Backtrace::capture(),
            };
        })?;
        tracing::info!(
            "ubpublish stream success, stream_name: {}, app: {} total stream count: {}",
            &stream_id.stream_name,
            &stream_id.app,
            self.streams.len()
        );
        Ok(())
    }

    async fn process_subscribe_event(
        &mut self,
        stream_id: StreamIdentifier,
        result_sender: Sender<StreamCenterResult<(Uuid, StreamType, mpsc::Receiver<FrameData>)>>,
    ) -> StreamCenterResult<()> {
        if !self.streams.contains_key(&stream_id) {
            return result_sender
                .send(Err(StreamCenterError::StreamNotFound(stream_id.clone())))
                .await
                .map_err(|err| {
                    tracing::error!(
                        "deliver subscribe failed result to caller failed, {:?}",
                        err
                    );
                    return StreamCenterError::StreamNotFound(stream_id.clone());
                });
        }

        let (tx, rx) = mpsc::channel(128);
        let uuid = Uuid::now_v7();
        let stream_type;
        {
            let stream = self.streams.get_mut(&stream_id).expect("this must exist");
            stream
                .data_distributer
                .write()
                .await
                .insert(uuid.clone(), tx);
            stream_type = stream.stream_type;
        }

        result_sender
            .send(Ok((uuid, stream_type, rx)))
            .await
            .map_err(|err| {
                tracing::error!(
                    "deliver subscribe success result to caller failed, {:?}",
                    err
                );
                return StreamCenterError::ChannelSendFailed {
                    backtrace: Backtrace::capture(),
                };
            })?;
        tracing::info!(
            "subscribe stream success, stream_name: {}, app: {}, stream_type: {}, uuid: {}",
            &stream_id.stream_name,
            &stream_id.app,
            stream_type,
            uuid,
        );
        Ok(())
    }

    async fn process_unsubscribe_event(
        &mut self,
        uuid: Uuid,
        stream_id: StreamIdentifier,
        result_sender: Sender<StreamCenterResult<()>>,
    ) -> StreamCenterResult<()> {
        if !self.streams.contains_key(&stream_id) {
            return result_sender
                .send(Err(StreamCenterError::StreamNotFound(stream_id.clone())))
                .await
                .map_err(|err| {
                    tracing::error!(
                        "deliver unsubscribe fail result to caller failed, {:?}",
                        err
                    );
                    return StreamCenterError::StreamNotFound(stream_id.clone());
                });
        }
        {
            self.streams
                .get_mut(&stream_id)
                .expect("this must exist")
                .data_distributer
                .write()
                .await
                .remove(&uuid);
        }

        result_sender.send(Ok(())).await.map_err(|err| {
            tracing::error!(
                "deliver unsubscribe sucess result to caller failed, {:?}",
                err
            );
            return StreamCenterError::ChannelSendFailed {
                backtrace: Backtrace::capture(),
            };
        })?;
        tracing::info!(
            "unsubscribe stream success, stream_name: {}, app: {}, uuid: {}",
            &stream_id.stream_name,
            &stream_id.app,
            uuid,
        );
        Ok(())
    }
}
