use std::{backtrace::Backtrace, collections::HashMap, sync::Arc};

use tokio::sync::{
    RwLock,
    mpsc::{self},
    oneshot,
};
use uuid::Uuid;

use crate::{
    errors::{StreamCenterError, StreamCenterResult},
    events::{StreamCenterEvent, SubscribeResponse},
    frame_info::ChunkFrameData,
    signal::StreamSignal,
    stream_source::{ParsedContext, StreamIdentifier, StreamSource, StreamType, SubscribeHandler},
};

#[derive(Debug)]
pub struct StreamSourceDynamicInfo {
    pub has_video: bool,
    pub has_audio: bool,
}

#[derive(Debug)]
struct StreamSourceHandles {
    signal_sender: mpsc::Sender<StreamSignal>,
    _source_sender: mpsc::Sender<ChunkFrameData>,

    _stream_identifier: StreamIdentifier,
    stream_type: StreamType,

    data_distributer: Arc<RwLock<HashMap<Uuid, SubscribeHandler>>>,
    stream_dynamic_info: Arc<RwLock<StreamSourceDynamicInfo>>,
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
        tracing::info!("stream center is running");
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
            } => self.process_publish_event(stream_type, stream_id, context, result_sender)?,
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
                context,
            } => {
                self.process_subscribe_event(stream_id, result_sender, context)
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

    fn process_publish_event(
        &mut self,
        stream_type: StreamType,
        stream_id: StreamIdentifier,
        context: HashMap<String, String>,
        result_sender: oneshot::Sender<StreamCenterResult<mpsc::Sender<ChunkFrameData>>>,
    ) -> StreamCenterResult<()> {
        if self.streams.contains_key(&stream_id) {
            return result_sender
                .send(Err(StreamCenterError::DuplicateStream(stream_id.clone())))
                .map_err(|err| {
                    tracing::error!("deliver publish fail result to caller failed, {:?}", err);
                    StreamCenterError::ChannelSendFailed {
                        backtrace: Backtrace::capture(),
                    }
                });
        }

        let (frame_sender, frame_receiver) = mpsc::channel(128);
        let (signal_sender, signal_receiver) = mpsc::channel(1);
        let data_distributer = Arc::new(RwLock::new(HashMap::new()));
        let stream_source_dynamic_info = Arc::new(RwLock::new(StreamSourceDynamicInfo {
            has_video: true,
            has_audio: true,
        }));

        let mut source = StreamSource::new(
            &stream_id.stream_name,
            &stream_id.app,
            stream_type,
            frame_receiver,
            signal_receiver,
            Arc::clone(&data_distributer),
            Arc::clone(&stream_source_dynamic_info),
        );

        tokio::spawn(async move { source.run().await });

        self.streams.insert(
            stream_id.clone(),
            StreamSourceHandles {
                signal_sender,
                _source_sender: frame_sender.clone(),
                _stream_identifier: stream_id.clone(),
                stream_type,
                data_distributer,
                stream_dynamic_info: stream_source_dynamic_info,
            },
        );

        result_sender.send(Ok(frame_sender)).map_err(|err| {
            tracing::error!("deliver publish success result to caller failed, {:?}", err);
            StreamCenterError::ChannelSendFailed {
                backtrace: Backtrace::capture(),
            }
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
        result_sender: oneshot::Sender<StreamCenterResult<()>>,
    ) -> StreamCenterResult<()> {
        let removed = self.streams.remove(&stream_id);
        match removed {
            None => result_sender
                .send(Err(StreamCenterError::StreamNotFound(stream_id.clone())))
                .map_err(|err| {
                    tracing::error!(
                        "deliver unpublish failed result to caller failed, {:?}",
                        err
                    );
                    StreamCenterError::ChannelSendFailed {
                        backtrace: Backtrace::capture(),
                    }
                }),
            Some(handles) => {
                let _ = handles
                    .signal_sender
                    .send(StreamSignal::Stop)
                    .await
                    .map_err(|err| {
                        tracing::error!("send stop signal to stream source failed, {:?}", err);
                        StreamCenterError::ChannelSendFailed {
                            backtrace: Backtrace::capture(),
                        }
                    });

                result_sender.send(Ok(())).map_err(|err| {
                    tracing::error!(
                        "deliver unpublish success result to caller failed, {:?}",
                        err
                    );
                    StreamCenterError::ChannelSendFailed {
                        backtrace: Backtrace::capture(),
                    }
                })?;
                tracing::info!(
                    "ubpublish stream success, stream_name: {}, app: {} total stream count: {}",
                    &stream_id.stream_name,
                    &stream_id.app,
                    self.streams.len()
                );
                Ok(())
            }
        }
    }

    async fn process_subscribe_event(
        &mut self,
        stream_id: StreamIdentifier,
        result_sender: oneshot::Sender<StreamCenterResult<SubscribeResponse>>,
        context: HashMap<String, String>,
    ) -> StreamCenterResult<()> {
        if !self.streams.contains_key(&stream_id) {
            return result_sender
                .send(Err(StreamCenterError::StreamNotFound(stream_id.clone())))
                .map_err(|err| {
                    tracing::error!(
                        "deliver subscribe failed result to caller failed, {:?}",
                        err
                    );
                    StreamCenterError::StreamNotFound(stream_id.clone())
                });
        }

        let (tx, rx) = mpsc::channel(100_000);
        let uuid = Uuid::now_v7();
        let stream_type;
        let source_has_video;
        let source_has_audio;
        {
            let parsed_context: ParsedContext = (&context).into();
            let stream = self.streams.get_mut(&stream_id).expect("this must exist");
            stream.data_distributer.write().await.insert(
                uuid,
                SubscribeHandler {
                    context,
                    parsed_context,
                    data_sender: tx,
                    stat: Default::default(),
                },
            );
            stream_type = stream.stream_type;
            let info = stream.stream_dynamic_info.read().await;
            source_has_video = info.has_video;
            source_has_audio = info.has_audio;
        }

        result_sender
            .send(Ok(SubscribeResponse {
                subscribe_id: uuid,
                stream_type,
                has_video: source_has_video,
                has_audio: source_has_audio,
                media_receiver: rx,
            }))
            .map_err(|err| {
                tracing::error!(
                    "deliver subscribe success result to caller failed, {:?}",
                    err
                );
                StreamCenterError::ChannelSendFailed {
                    backtrace: Backtrace::capture(),
                }
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
        result_sender: oneshot::Sender<StreamCenterResult<()>>,
    ) -> StreamCenterResult<()> {
        if !self.streams.contains_key(&stream_id) {
            return result_sender
                .send(Err(StreamCenterError::StreamNotFound(stream_id.clone())))
                .map_err(|err| {
                    tracing::error!(
                        "deliver unsubscribe fail result to caller failed, {:?}",
                        err
                    );
                    StreamCenterError::StreamNotFound(stream_id.clone())
                });
        }
        {
            let removed = self
                .streams
                .get_mut(&stream_id)
                .expect("this must exist")
                .data_distributer
                .write()
                .await
                .remove(&uuid);
            if let Some(handler) = removed {
                tracing::info!("unsubscribe done, stat: {:?}", handler.stat);
            }
        }

        result_sender.send(Ok(())).map_err(|err| {
            tracing::error!(
                "deliver unsubscribe success result to caller failed, {:?}",
                err
            );
            StreamCenterError::ChannelSendFailed {
                backtrace: Backtrace::capture(),
            }
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

impl Default for StreamCenter {
    fn default() -> Self {
        Self::new()
    }
}
