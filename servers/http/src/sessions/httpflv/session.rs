use std::{collections::HashMap, io};

use byteorder::{BigEndian, WriteBytesExt};

use codec_common::video::VideoConfig;
use flv_formats::{header::FLVHeader, tag::flv_tag_header::FLVTagHeader};
use num::ToPrimitive;
use stream_center::{
    events::{StreamCenterEvent, SubscribeResponse},
    gop::MediaFrame,
    stream_center::StreamCenter,
    stream_source::{PlayProtocol, StreamIdentifier, StreamType},
};
use tokio::sync::mpsc;
use tokio_util::bytes::BytesMut;
use utils::traits::fixed_packet::FixedPacket;
use utils::traits::writer::WriteTo;
use uuid::Uuid;

use crate::routes::params::{AUDIO_ONLY_KEY, VIDEO_ONLY_KEY};

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

#[derive(Debug)]
pub struct HttpFlvSession {
    _config: HttpFlvSessionConfig,
    stream_center_event_sender: mpsc::UnboundedSender<StreamCenterEvent>,
    stream_properties: StreamProperties,
    play_id: Option<Uuid>,
    http_response_bytes_sender: mpsc::UnboundedSender<BytesMut>,
    nalu_length_size: Option<u8>,

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
            _config: config.clone(),
            nalu_length_size: None,
            stream_center_event_sender,
            stream_properties,
            play_id: None,
            http_response_bytes_sender,
            has_audio: true,
            has_video: true,
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
            .get(AUDIO_ONLY_KEY)
            .map_or_else(|| true, |_| false)
            && response.has_video;
        self.has_audio = self
            .stream_properties
            .stream_context
            .get(VIDEO_ONLY_KEY)
            .map_or_else(|| true, |_| false)
            && response.has_audio;

        let mut bytes = Vec::with_capacity(4096);

        {
            let flv_file_header = FLVHeader::new(self.has_audio, self.has_video);
            bytes.reserve(9 + 4);
            flv_file_header.write_to(&mut bytes)?;
            bytes.write_u32::<BigEndian>(0)?;
        }

        let mut has_video_sequence_header = !self.has_video;
        let mut has_audio_sequence_header = !self.has_audio;
        loop {
            match response.media_receiver.recv().await {
                None => {}
                Some(frame) => {
                    if !has_audio_sequence_header {
                        has_audio_sequence_header = frame.is_audio() && frame.is_sequence_header();
                    }
                    if !has_video_sequence_header {
                        has_video_sequence_header = frame.is_video() && frame.is_sequence_header();
                    }

                    if !has_audio_sequence_header
                        && !has_video_sequence_header
                        && !frame.is_script()
                    {
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

    pub fn write_flv_tag<W: io::Write>(
        &mut self,
        frame: MediaFrame,
        bytes_buffer: &mut W,
    ) -> HttpFlvSessionResult<()> {
        if let MediaFrame::VideoConfig {
            timestamp_nano: _,
            config,
        } = &frame
        {
            self.nalu_length_size = match config.as_ref() {
                VideoConfig::H264 {
                    sps: _,
                    pps: _,
                    sps_ext: _,
                    avc_decoder_configuration_record,
                } => avc_decoder_configuration_record
                    .as_ref()
                    .map(|v| v.length_size_minus_one.checked_add(1).unwrap()),
            }
        }
        let tag = frame.to_flv_tag(self.nalu_length_size.unwrap_or(4))?;
        tag.write_to(bytes_buffer)?;
        bytes_buffer.write_u32::<BigEndian>(
            tag.tag_header
                .data_size
                .checked_add(FLVTagHeader::bytes_count().to_u32().unwrap())
                .unwrap(),
        )?;
        Ok(())
    }

    pub async fn unsubscribe_from_stream_center(&self) -> HttpFlvSessionResult<()> {
        if self.play_id.is_none() {
            return Ok(());
        }
        StreamCenter::unsubscribe(
            &self.stream_center_event_sender,
            self.play_id.unwrap(),
            &StreamIdentifier {
                stream_name: self.stream_properties.stream_name.clone(),
                app: self.stream_properties.app.clone(),
            },
        )
        .await
        .map_err(|err| err.into())
    }

    pub async fn subscribe_from_stream_center(&self) -> HttpFlvSessionResult<SubscribeResponse> {
        StreamCenter::subscribe(
            &self.stream_center_event_sender,
            &StreamIdentifier {
                stream_name: self.stream_properties.stream_name.clone(),
                app: self.stream_properties.app.clone(),
            },
            PlayProtocol::HTTPFLV,
            &self.stream_properties.stream_context,
        )
        .await
        .map_err(|err| err.into())
    }
}
