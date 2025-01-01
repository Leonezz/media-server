use std::{collections::HashMap, io::Write};

use byteorder::{BigEndian, WriteBytesExt};

use flv::{
    errors::FLVResult,
    header::FLVHeader,
    tag::{FLVTagHeader, FLVTagType},
};
use stream_center::{
    events::{StreamCenterEvent, SubscribeResponse},
    gop::FLVMediaFrame,
    stream_source::{StreamIdentifier, StreamType},
};
use tokio::sync::{mpsc, oneshot};
use tokio_util::bytes::BytesMut;
use utils::system::time::get_timestamp_ns;
use uuid::Uuid;

use crate::sessions::httpflv::errors::HttpFlvSessionError;

use super::errors::HttpFlvSessionResult;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HttpFlvSessionConfig {
    pub chunk_size: u32,
    pub write_timeout_ms: u64,
    pub read_timeout_ms: u64,
}

#[derive(Debug, Default)]
pub struct StreamProperties {
    pub stream_name: String,
    pub app: String,
    pub stream_type: StreamType,
    pub stream_context: HashMap<String, String>,
}

#[derive(Debug, Default)]
pub struct HttpFlvSessionRuntimeStat {
    video_sequence_header_sent: bool,
    audio_sequence_header_sent: bool,
    video_frame_sent: u64,
    audio_frame_sent: u64,
    video_frame_send_failed: u64,
    audio_frame_send_failed: u64,
}

#[derive(Debug)]
pub struct HttpFlvSession {
    config: HttpFlvSessionConfig,
    stream_center_event_sender: mpsc::UnboundedSender<StreamCenterEvent>,
    stream_properties: StreamProperties,
    play_id: Option<Uuid>,
    http_response_bytes_sender: mpsc::UnboundedSender<BytesMut>,

    runtime_stat: HttpFlvSessionRuntimeStat,
    has_video: bool,
    has_audio: bool,
}

impl HttpFlvSession {
    pub fn new(
        config: HttpFlvSessionConfig,
        stream_center_event_sender: mpsc::UnboundedSender<StreamCenterEvent>,
        stream_properties: StreamProperties,
        http_response_bytes_sender: mpsc::UnboundedSender<BytesMut>,
    ) -> Self {
        Self {
            config: config.clone(),
            stream_center_event_sender,
            stream_properties,
            play_id: None,
            http_response_bytes_sender,
            has_audio: true,
            has_video: true,

            runtime_stat: Default::default(),
        }
    }

    pub async fn serve_pull_request(
        &mut self,
        mut response: SubscribeResponse,
    ) -> HttpFlvSessionResult<()> {
        self.stream_properties.stream_type = response.stream_type;
        self.play_id = Some(response.subscribe_id);
        self.has_video = self
            .stream_properties
            .stream_context
            .get("audioOnly")
            .map_or_else(|| true, |_| false)
            && response.has_video;
        self.has_audio = self
            .stream_properties
            .stream_context
            .get("videoOnly")
            .map_or_else(|| true, |_| false)
            && response.has_audio;

        let mut bytes = Vec::with_capacity(4096);

        {
            let flv_file_header = FLVHeader::new(self.has_audio, self.has_video);
            bytes.reserve(9 + 4);
            flv_file_header.write_to(&mut bytes)?;
            bytes.write_u32::<BigEndian>(0)?;
        }

        let mut has_video_sequence_header = !self.has_video || false;
        let mut has_audio_sequence_header = !self.has_audio || false;
        loop {
            match response.media_receiver.recv().await {
                None => {}
                Some(frame) => {
                    match &frame {
                        FLVMediaFrame::Video {
                            runtime_stat: _,
                            pts: _,
                            header,
                            payload: _,
                        } => {
                            if header.is_sequence_header() {
                                has_video_sequence_header = true;
                                self.runtime_stat.video_sequence_header_sent = true;
                            }
                        }
                        FLVMediaFrame::Audio {
                            runtime_stat: _,
                            pts: _,
                            header,
                            payload: _,
                        } => {
                            if header.is_sequence_header() {
                                has_audio_sequence_header = true;
                                self.runtime_stat.audio_sequence_header_sent = true;
                            }
                        }
                        _ => {}
                    };

                    if !has_audio_sequence_header && !has_video_sequence_header {
                        // we hold until sequence header
                        continue;
                    }

                    self.write_flv_tag(frame, &mut bytes)?;

                    let res = self
                        .http_response_bytes_sender
                        .send(BytesMut::from(&bytes[..]));
                    bytes.clear();
                    if res.is_err() {
                        tracing::error!(
                            "send http response bytes to http request handler failed: {:?},
                            the receiver must been closed, which means the consumer must have unsubscribed. stream: {:?}",
                            res,
                            self.stream_properties
                        );
                        return Ok(());
                    }
                }
            };
        }
    }

    pub fn write_flv_tag(
        &mut self,
        mut frame: FLVMediaFrame,
        bytes_buffer: &mut Vec<u8>,
    ) -> HttpFlvSessionResult<()> {
        fn write_tag(
            tag_type: FLVTagType,
            timestamp: u32,
            payload: &BytesMut,
            bytes_buffer: &mut Vec<u8>,
        ) -> FLVResult<()> {
            let flv_tag_header = FLVTagHeader {
                tag_type,
                data_size: payload.len() as u32,
                timestamp,
                filter_enabled: false,
            };

            const FLV_TAG_HEADER_SIZE: usize = 11;
            const FLV_PREV_TAG_SIZE_BYTES: usize = 4;

            bytes_buffer.reserve(
                FLV_TAG_HEADER_SIZE + FLV_PREV_TAG_SIZE_BYTES + flv_tag_header.data_size as usize,
            );
            flv_tag_header.write_to(bytes_buffer.by_ref())?;
            bytes_buffer.extend_from_slice(&payload[..]);

            // write prev tag size
            bytes_buffer
                .write_u32::<BigEndian>(flv_tag_header.data_size + FLV_TAG_HEADER_SIZE as u32)?;
            Ok(())
        }
        match &mut frame {
            FLVMediaFrame::Video {
                runtime_stat,
                pts,
                header,
                payload,
            } => {
                runtime_stat.play_time_ns = get_timestamp_ns().expect("this cannot be error");
                self.runtime_stat.video_frame_sent += 1;

                let timestamp =
                    *pts + if let Some(cts) = header.composition_time {
                        cts as u64
                    } else {
                        0
                    } + if let Some(cts_nano) = header.timestamp_nano {
                        cts_nano as u64
                    } else {
                        0
                    };

                write_tag(FLVTagType::Video, timestamp as u32, payload, bytes_buffer)?;
            }
            FLVMediaFrame::Audio {
                runtime_stat,
                pts,
                header: _,
                payload,
            } => {
                runtime_stat.play_time_ns = get_timestamp_ns().expect("this cannot be error");
                self.runtime_stat.audio_frame_sent += 1;

                write_tag(FLVTagType::Audio, *pts as u32, payload, bytes_buffer)?;
            }
            FLVMediaFrame::Script {
                runtime_stat,
                pts,
                payload,
                on_meta_data: _,
            } => {
                runtime_stat.play_time_ns = get_timestamp_ns().expect("this cannot be error");

                write_tag(FLVTagType::Script, *pts as u32, payload, bytes_buffer)?;
            }
        }
        Ok(())
    }

    pub async fn unsubscribe_from_stream_center(&self) -> HttpFlvSessionResult<()> {
        if self.play_id.is_none() {
            return Ok(());
        }
        let (tx, rx) = oneshot::channel();
        let event = StreamCenterEvent::Unsubscribe {
            stream_id: StreamIdentifier {
                stream_name: self.stream_properties.stream_name.clone(),
                app: self.stream_properties.app.clone(),
            },
            uuid: self.play_id.expect("this cannot be none"),
            result_sender: tx,
        };

        let res = self.stream_center_event_sender.send(event);
        if res.is_err() {
            tracing::error!(
                "unsubscribe from stream center failed, stream: {:?}",
                self.stream_properties
            );
            return Err(HttpFlvSessionError::StreamEventSendFailed(Some(
                res.expect_err("this must be error").0,
            )));
        }

        match rx.await {
            Err(_err) => {
                tracing::error!(
                    "channel closed while trying receive unsubscribe result, stream: {:?}",
                    self.stream_properties
                );
                return Err(HttpFlvSessionError::StreamEventSendFailed(None));
            }
            Ok(Err(err)) => {
                tracing::error!(
                    "unsubscribe from stream center failed, {:?}, stream: {:?}",
                    err,
                    self.stream_properties
                );
                return Err(err.into());
            }
            Ok(Ok(())) => {
                tracing::info!(
                    "unsubscribe from stream center succeed, stream: {:?}, uuid: {:?}",
                    self.stream_properties,
                    self.play_id
                );
                return Ok(());
            }
        }
    }

    pub async fn subscribe_from_stream_center(&self) -> HttpFlvSessionResult<SubscribeResponse> {
        let (tx, rx) = oneshot::channel();
        let event = StreamCenterEvent::Subscribe {
            stream_id: StreamIdentifier {
                stream_name: self.stream_properties.stream_name.clone(),
                app: self.stream_properties.app.clone(),
            },
            context: self.stream_properties.stream_context.clone(),
            result_sender: tx,
        };
        let res = self.stream_center_event_sender.send(event);

        if res.is_err() {
            tracing::error!(
                "subscribe from stream center failed, stream: {:?}",
                self.stream_properties,
            );
            return Err(HttpFlvSessionError::StreamEventSendFailed(Some(
                res.expect_err("this must be an error").0,
            )));
        }

        match rx.await {
            Err(_err) => {
                tracing::error!(
                    "channel closed while trying receive subscribe result, stream: {:?}",
                    self.stream_properties,
                );
                return Err(HttpFlvSessionError::StreamEventSendFailed(None));
            }
            Ok(Err(err)) => {
                tracing::error!(
                    "subscribe from stream center failed, {:?}, stream: {:?}",
                    err,
                    self.stream_properties
                );
                return Err(err.into());
            }
            Ok(Ok(res)) => {
                tracing::info!(
                    "subscribe from stream center success, stream: {:?}, uuid: {}",
                    self.stream_properties,
                    res.subscribe_id
                );
                Ok(res)
            }
        }
    }
}
