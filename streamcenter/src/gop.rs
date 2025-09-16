use crate::errors::{StreamCenterError, StreamCenterResult};
use bitstream_io::{BitRead, BitWrite};
use codec_aac::mpeg4_configuration::audio_specific_config::AudioSpecificConfig;
use codec_common::{
    FrameType, MediaFrameTimestamp,
    audio::{AudioCodecCommon, AudioConfig, AudioFrameInfo, SoundInfoCommon},
    video::{H264VideoConfig, VideoCodecCommon, VideoConfig, VideoFrameInfo, VideoFrameUnit},
};
use codec_h264::avc_decoder_configuration_record::AvcDecoderConfigurationRecord;
use flv_formats::tag::{
    FLVTag,
    audio_tag_header::LegacyAudioTagHeader,
    audio_tag_header_info::AudioTagHeaderWithoutMultiTrack,
    enhanced::ex_video::ex_video_header::VideoPacketType,
    flv_tag_body::FLVTagBody,
    flv_tag_header::FLVTagType,
    on_meta_data::OnMetaData,
    video_tag_header::{FrameTypeFLV, LegacyVideoTagHeader},
    video_tag_header_info::VideoTagHeaderWithoutMultiTrack,
};
use num::ToPrimitive;
use std::{
    collections::{HashMap, VecDeque},
    io,
};
use tokio_util::bytes::{Buf, Bytes};
use tracing::debug_span;
use utils::traits::reader::ReadFrom;
use utils::traits::writer::{BitwiseWriteTo, WriteTo};
use utils::traits::{
    dynamic_sized_packet::{DynamicSizedBitsPacket, DynamicSizedPacket},
    reader::BitwiseReadFrom,
};

#[derive(Debug, Clone)]
pub enum MediaFrame {
    VideoConfig {
        timestamp_nano: u64,
        config: Box<VideoConfig>,
    },
    Video {
        frame_info: VideoFrameInfo,
        payload: VideoFrameUnit,
    },
    AudioConfig {
        timestamp_nano: u64,
        sound_info: SoundInfoCommon,
        config: Box<AudioConfig>,
    },
    Audio {
        // NOTE - this tag_header is also included in the frame payload
        frame_info: AudioFrameInfo,
        payload: Bytes,
    },
    Script {
        timestamp_nano: u64,
        // onMetaData should be the content of payload,
        // note the payload still holds all the bytes
        on_meta_data: Box<Option<OnMetaData>>,
        payload: Bytes,
    },
}

impl MediaFrame {
    #[inline]
    pub fn is_video(&self) -> bool {
        matches!(
            self,
            MediaFrame::Video {
                frame_info: _,
                payload: _,
            } | MediaFrame::VideoConfig { .. }
        )
    }

    #[inline]
    pub fn get_codec_name(&self) -> &'static str {
        if self.is_audio() {
            match self.audio_codec_id() {
                Some(id) => id.get_codec_name(),
                None => "Known",
            }
        } else if self.is_video() {
            match self.video_codec_id() {
                Some(id) => id.get_codec_name(),
                None => "Known",
            }
        } else {
            "Known"
        }
    }

    #[inline]
    pub fn is_audio(&self) -> bool {
        matches!(
            self,
            MediaFrame::Audio {
                frame_info: _,
                payload: _,
            } | MediaFrame::AudioConfig { .. }
        )
    }

    #[inline]
    pub fn is_script(&self) -> bool {
        matches!(
            self,
            MediaFrame::Script {
                timestamp_nano: _,
                payload: _,
                on_meta_data: _,
            }
        )
    }

    pub fn get_presentation_timestamp_ns(&self) -> u64 {
        match self {
            Self::Audio {
                frame_info: AudioFrameInfo { timestamp_nano, .. },
                ..
            }
            | Self::AudioConfig { timestamp_nano, .. }
            | Self::VideoConfig { timestamp_nano, .. }
            | Self::Script { timestamp_nano, .. } => *timestamp_nano,
            Self::Video {
                frame_info: VideoFrameInfo { timestamp, .. },
                ..
            } => timestamp.pts(),
        }
    }

    pub fn get_presentation_timestamp_ms(&self) -> u64 {
        self.get_presentation_timestamp_ns()
            .checked_div(1_000_000)
            .unwrap()
    }

    pub fn get_decode_timestamp_ns(&self) -> u64 {
        match self {
            Self::Audio {
                frame_info: AudioFrameInfo { timestamp_nano, .. },
                ..
            }
            | Self::AudioConfig { timestamp_nano, .. }
            | Self::VideoConfig { timestamp_nano, .. }
            | Self::Script { timestamp_nano, .. } => *timestamp_nano,
            Self::Video {
                frame_info: VideoFrameInfo { timestamp, .. },
                ..
            } => timestamp.dts(),
        }
    }

    pub fn get_decode_timestamp_ms(&self) -> u64 {
        self.get_decode_timestamp_ns()
            .checked_div(1_000_000)
            .unwrap()
    }

    pub fn set_presentation_timestamp_ns(&mut self, pts_nano: u64) {
        match self {
            Self::Audio {
                frame_info: AudioFrameInfo { timestamp_nano, .. },
                ..
            }
            | Self::AudioConfig { timestamp_nano, .. }
            | Self::VideoConfig { timestamp_nano, .. }
            | Self::Script { timestamp_nano, .. } => *timestamp_nano = pts_nano,
            Self::Video {
                frame_info: VideoFrameInfo { timestamp, .. },
                ..
            } => {
                timestamp.set_pts(pts_nano);
            }
        }
    }

    pub fn set_presentation_timestamp_ms(&mut self, pts_ms: u64) {
        let ts = pts_ms.checked_mul(1_000_000).unwrap();
        self.set_presentation_timestamp_ns(ts);
    }

    pub fn set_decode_timestamp_ns(&mut self, dts_nano: u64) {
        match self {
            Self::Audio {
                frame_info: AudioFrameInfo { timestamp_nano, .. },
                ..
            }
            | Self::AudioConfig { timestamp_nano, .. }
            | Self::VideoConfig { timestamp_nano, .. }
            | Self::Script { timestamp_nano, .. } => *timestamp_nano = dts_nano,
            Self::Video {
                frame_info: VideoFrameInfo { timestamp, .. },
                ..
            } => {
                timestamp.set_dts(dts_nano);
            }
        }
    }

    pub fn set_decode_timestamp_ms(&mut self, dts_ms: u64) {
        let ts = dts_ms.checked_mul(1_000_000).unwrap();
        self.set_decode_timestamp_ns(ts);
    }

    #[inline]
    pub fn is_sequence_header(&self) -> bool {
        matches!(
            self,
            MediaFrame::AudioConfig { .. } | MediaFrame::VideoConfig { .. }
        )
    }

    #[inline]
    pub fn is_video_key_frame(&self) -> bool {
        match self {
            MediaFrame::Video {
                frame_info,
                payload: _,
            } => frame_info.frame_type == FrameType::KeyFrame,
            _ => false,
        }
    }

    pub fn video_codec_id(&self) -> Option<VideoCodecCommon> {
        match self {
            Self::Video {
                frame_info,
                payload: _,
            } => Some(frame_info.codec_id),
            _ => None,
        }
    }

    pub fn audio_codec_id(&self) -> Option<AudioCodecCommon> {
        match self {
            Self::Audio {
                frame_info,
                payload: _,
            } => Some(frame_info.codec_id),
            _ => None,
        }
    }

    pub fn same_codec_video(&self, other: &Self) -> bool {
        if !self.is_video() || !other.is_video() {
            return false;
        }
        self.video_codec_id() == other.video_codec_id()
    }

    pub fn to_flv_tag(&self, nalu_size_length: u8) -> StreamCenterResult<flv_formats::tag::FLVTag> {
        assert!(nalu_size_length == 1 || nalu_size_length == 2 || nalu_size_length == 4);
        let span = debug_span!("media frame to flv tag", nalu_size_length);

        let flv_dts_ms = self.get_decode_timestamp_ms().to_u32().unwrap();
        let _enter = span.enter();
        match self {
            Self::Audio {
                frame_info,
                payload,
            } => {
                let span = debug_span!("audio", ?frame_info);
                let _enter = span.enter();
                let legacy_header: LegacyAudioTagHeader = frame_info.try_into()?;
                Ok(flv_formats::tag::FLVTag {
                    tag_header: flv_formats::tag::flv_tag_header::FLVTagHeader {
                        tag_type: FLVTagType::Audio,
                        data_size: legacy_header
                            .get_packet_bytes_count()
                            .checked_add(payload.len())
                            .and_then(|v| v.to_u32())
                            .unwrap(),
                        timestamp: flv_dts_ms,
                        filter_enabled: false,
                    },
                    body_with_filter: flv_formats::tag::flv_tag_body::FLVTagBodyWithFilter {
                        filter: None,
                        body: flv_formats::tag::flv_tag_body::FLVTagBody::Audio {
                            header: flv_formats::tag::audio_tag_header::AudioTagHeader::Legacy(
                                legacy_header,
                            ),
                            body: payload.clone(),
                        },
                    },
                })
            }
            Self::Script { on_meta_data, .. } => {
                let value = vec![
                    // amf_formats::amf0::string("@setDataFrame"),
                    amf_formats::amf0::string("onMetaData"),
                    amf_formats::amf0::Value::ECMAArray(
                        on_meta_data.clone().map_or(vec![], |ref v| v.into()),
                    ),
                ];
                let length = value.iter().fold(0, |prev, item| {
                    let mut bytes = Vec::new();
                    item.write_to(&mut bytes).unwrap();
                    prev + bytes.len()
                });
                Ok(flv_formats::tag::FLVTag {
                    tag_header: flv_formats::tag::flv_tag_header::FLVTagHeader {
                        tag_type: FLVTagType::Script,
                        data_size: length.to_u32().unwrap(),
                        timestamp: flv_dts_ms,
                        filter_enabled: false,
                    },
                    body_with_filter: flv_formats::tag::flv_tag_body::FLVTagBodyWithFilter {
                        filter: None,
                        body: flv_formats::tag::flv_tag_body::FLVTagBody::Script { value },
                    },
                })
            }
            Self::Video {
                frame_info,
                payload,
            } => {
                let span = debug_span!("video", ?frame_info);
                let _enter = span.enter();
                let legacy_header: LegacyVideoTagHeader = frame_info.try_into()?;
                let mut bytes =
                    Vec::with_capacity(payload.bytes_cnt(nalu_size_length.to_usize().unwrap()));
                let mut writer = io::Cursor::new(&mut bytes);
                codec_common::video::writer::VideoFrameUnitAvccWriter(payload, nalu_size_length)
                    .write_to(&mut writer)
                    .map_err(|err| {
                        StreamCenterError::RemuxFailed(format!(
                            "remux from video frame to flv video tag failed: {}",
                            err
                        ))
                    })?;
                let tag_header = flv_formats::tag::flv_tag_header::FLVTagHeader {
                    tag_type: FLVTagType::Video,
                    data_size: legacy_header
                        .get_packet_bytes_count()
                        .checked_add(bytes.len())
                        .and_then(|v| v.to_u32())
                        .unwrap(),
                    timestamp: flv_dts_ms,
                    filter_enabled: false,
                };

                Ok(flv_formats::tag::FLVTag {
                    tag_header,
                    body_with_filter: flv_formats::tag::flv_tag_body::FLVTagBodyWithFilter {
                        filter: None,
                        body: flv_formats::tag::flv_tag_body::FLVTagBody::Video {
                            header: flv_formats::tag::video_tag_header::VideoTagHeader::Legacy(
                                legacy_header,
                            ),
                            body: Bytes::from_owner(bytes),
                        },
                    },
                })
            }
            Self::VideoConfig {
                timestamp_nano,
                config,
                ..
            } => {
                let frame_info = VideoFrameInfo {
                    codec_id: config.as_ref().into(),
                    frame_type: FrameType::SequenceStart,
                    timestamp: MediaFrameTimestamp::with_timestamp_nano(*timestamp_nano),
                };
                let span = debug_span!("video_config", ?frame_info);
                let _enter = span.enter();
                let legacy_header: LegacyVideoTagHeader = (&frame_info).try_into()?;
                match config.as_ref() {
                    VideoConfig::H264(H264VideoConfig {
                        sps: _,
                        pps: _,
                        sps_ext: _,
                        avc_decoder_configuration_record,
                    }) => {
                        if let Some(record) = avc_decoder_configuration_record {
                            let mut bytes = Vec::with_capacity(record.get_packet_bytes_count());
                            let mut writer = io::Cursor::new(&mut bytes);
                            record.write_to(&mut writer)?;
                            let tag_header = flv_formats::tag::flv_tag_header::FLVTagHeader {
                                tag_type: FLVTagType::Video,
                                data_size: legacy_header
                                    .get_packet_bytes_count()
                                    .checked_add(bytes.len())
                                    .and_then(|v| v.to_u32())
                                    .unwrap(),
                                timestamp: flv_dts_ms,
                                filter_enabled: false,
                            };
                            tracing::debug!("video sequence header tag header: {:?}", tag_header);
                            Ok(flv_formats::tag::FLVTag {
                                tag_header,
                                body_with_filter:
                                    flv_formats::tag::flv_tag_body::FLVTagBodyWithFilter {
                                        filter: None,
                                        body: flv_formats::tag::flv_tag_body::FLVTagBody::Video {
                                            header: flv_formats::tag::video_tag_header::VideoTagHeader::Legacy(legacy_header),
                                            body: Bytes::from_owner(bytes),
                                        },
                                    },
                            })
                        } else {
                            unimplemented!()
                        }
                    }
                }
            }
            Self::AudioConfig {
                timestamp_nano,
                sound_info,
                config,
            } => {
                let frame_info = AudioFrameInfo {
                    codec_id: config.as_ref().into(),
                    frame_type: FrameType::SequenceStart,
                    timestamp_nano: *timestamp_nano,
                    sound_info: *sound_info,
                };
                let span = debug_span!("audio_config", ?frame_info);
                let _enter = span.enter();
                let legacy_header: LegacyAudioTagHeader = (&frame_info).try_into()?;
                match config.as_ref() {
                    AudioConfig::AAC(config) => {
                        let mut bytes = Vec::with_capacity(
                            config
                                .get_packet_bits_count()
                                .checked_add(4)
                                .and_then(|v| v.checked_div(8))
                                .unwrap(),
                        );
                        let mut writer =
                            bitstream_io::BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
                        config.write_to(&mut writer)?;
                        writer.byte_align()?;
                        Ok(flv_formats::tag::FLVTag {
                            tag_header: flv_formats::tag::flv_tag_header::FLVTagHeader {
                                tag_type: FLVTagType::Audio,
                                data_size: legacy_header
                                    .get_packet_bytes_count()
                                    .checked_add(bytes.len())
                                    .and_then(|v| v.to_u32())
                                    .unwrap(),
                                timestamp: flv_dts_ms,
                                filter_enabled: false,
                            },
                            body_with_filter:
                                flv_formats::tag::flv_tag_body::FLVTagBodyWithFilter {
                                    filter: None,
                                    body: flv_formats::tag::flv_tag_body::FLVTagBody::Audio {
                                        header: flv_formats::tag::audio_tag_header::AudioTagHeader::Legacy(legacy_header),
                                        body: Bytes::from_owner(bytes),
                                    },
                                },
                        })
                    }
                }
            }
        }
    }

    pub fn from_flv_tag(tag: FLVTag, nalu_size_length: u8) -> StreamCenterResult<Self> {
        let span = tracing::debug_span!(
            "flv tag to media frame",
            packet_type=?tag.tag_header.tag_type,
            data_size=tag.tag_header.data_size,
            timestamp=tag.tag_header.timestamp,
        );
        let _enter = span.enter();
        match tag.body_with_filter.body {
            FLVTagBody::Audio { header, body } => {
                let tag_header_info: AudioTagHeaderWithoutMultiTrack = (&header).try_into()?;
                let span = tracing::debug_span!(
                    "audio",
                    packet_type=?tag_header_info.packet_type,
                    codec=?tag_header_info.codec_id,
                    timestamp_nano=tag_header_info.timestamp_nano.unwrap_or(0),
                );
                let _ = span.enter();
                let frame_info = AudioFrameInfo::new(
                    tag_header_info.codec_id,
                    tag_header_info.packet_type.try_into()?,
                    tag_header_info
                        .legacy_info
                        .unwrap_or_default()
                        .sound_rate
                        .into(),
                    tag_header_info
                        .legacy_info
                        .unwrap_or_default()
                        .sound_size
                        .into(),
                    tag_header_info
                        .legacy_info
                        .unwrap_or_default()
                        .sound_type
                        .into(),
                    tag.tag_header
                        .timestamp
                        .to_u64()
                        .and_then(|v| v.checked_mul(1_000_000))
                        .and_then(|v| {
                            v.checked_add(
                                tag_header_info
                                    .timestamp_nano
                                    .unwrap_or(0)
                                    .to_u64()
                                    .unwrap(),
                            )
                        })
                        .unwrap(),
                );
                if frame_info.frame_type == FrameType::SequenceStart {
                    let audio_config = match frame_info.codec_id {
                        codec_common::audio::AudioCodecCommon::AAC => {
                            let mut reader = codec_bitstream::reader::BitstreamReader::new(&body);
                            AudioConfig::AAC(AudioSpecificConfig::read_from(reader.by_ref())?)
                        }
                        _ => {
                            todo!()
                        }
                    };
                    tracing::debug!("got audio config: {:?}", audio_config);
                    return Ok(Self::AudioConfig {
                        timestamp_nano: 0,
                        sound_info: frame_info.sound_info,
                        config: Box::new(audio_config),
                    });
                }
                Ok(Self::Audio {
                    frame_info,
                    payload: body,
                })
            }
            FLVTagBody::Script { ref value } => {
                let mut bytes = Vec::new();
                tag.body_with_filter.write_to(&mut bytes)?;

                let mut map = HashMap::new();
                for v in value {
                    let pairs = v.clone().try_into_pairs();
                    if let Ok(pairs) = pairs {
                        for (k, v) in pairs {
                            map.insert(k, v);
                        }
                    }
                }

                Ok(Self::Script {
                    timestamp_nano: tag
                        .tag_header
                        .timestamp
                        .to_u64()
                        .and_then(|v| v.checked_mul(1_000_000))
                        .unwrap(),
                    on_meta_data: Box::new(Some(OnMetaData::from(map))),
                    payload: bytes.into(),
                })
            }
            FLVTagBody::Video { header, body } => {
                let tag_header_info: VideoTagHeaderWithoutMultiTrack = (&header).try_into()?;
                let span = tracing::debug_span!(
                    "video",
                    packet_type=?tag_header_info.packet_type,
                    codec=?tag_header_info.codec_id,
                    frame_type=?tag_header_info.frame_type,
                    cts=tag_header_info.composition_time.unwrap_or(0),
                    timestamp_nano=tag_header_info.timestamp_nano.unwrap_or(0),
                );
                let _enter = span.enter();
                match tag_header_info.packet_type {
                    VideoPacketType::SequenceStart => {
                        // avc decoder configuration record
                        let video_config = match tag_header_info.codec_id {
                            VideoCodecCommon::AVC => {
                                let config =
                                    AvcDecoderConfigurationRecord::read_from(&mut body.reader())?;
                                VideoConfig::from(config)
                            }
                            _ => {
                                todo!()
                            }
                        };
                        tracing::debug!("got video config: {:#?}", video_config);
                        Ok(Self::VideoConfig {
                            timestamp_nano: tag
                                .tag_header
                                .timestamp
                                .to_u64()
                                .and_then(|v| v.checked_mul(1_000_000))
                                .unwrap(),
                            config: Box::new(video_config),
                        })
                    }
                    VideoPacketType::MPEG2TSSequenceStart => {
                        unimplemented!()
                    }
                    _ => {
                        let nalus = codec_common::video::reader::parse_to_nal_units(
                            &body,
                            tag_header_info.codec_id,
                            Some(nalu_size_length),
                        )
                        .map_err(|err| {
                            StreamCenterError::RemuxFailed(format!(
                                "demux video nalus for codec id: {:?} failed: {}",
                                tag_header_info.codec_id, err
                            ))
                        })?;
                        let frame_type =
                            if tag_header_info.packet_type == VideoPacketType::SequenceEnd {
                                FrameType::SequenceEnd
                            } else if tag_header_info.frame_type == FrameTypeFLV::KeyFrame
                                || nalus.has_idr()
                            {
                                FrameType::KeyFrame
                            } else {
                                FrameType::CodedFrames
                            };
                        let timestamp = *MediaFrameTimestamp::with_timestamp_ms(
                            tag.tag_header.timestamp.to_u64().unwrap(),
                        )
                        .apply_offset_ms(
                            tag_header_info
                                .composition_time
                                .unwrap_or(0)
                                .to_u64()
                                .unwrap(),
                        )
                        .apply_offset_nano(
                            tag_header_info
                                .timestamp_nano
                                .unwrap_or(0)
                                .to_u64()
                                .unwrap(),
                        );
                        Ok(Self::Video {
                            frame_info: VideoFrameInfo::new(
                                tag_header_info.codec_id,
                                frame_type,
                                timestamp,
                            ),
                            payload: nalus,
                        })
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct Gop {
    pub media_frames: VecDeque<MediaFrame>,
    video_tag_cnt: usize,
    audio_tag_cnt: usize,
    meta_tag_cnt: usize,
    first_video_dts_nano: u64,
    last_video_dts_nano: u64,
}

impl Gop {
    pub fn new() -> Self {
        Self {
            media_frames: VecDeque::new(),
            video_tag_cnt: 0,
            audio_tag_cnt: 0,
            meta_tag_cnt: 0,
            first_video_dts_nano: 0,
            last_video_dts_nano: 0,
        }
    }

    #[inline]
    pub fn pop_front(&mut self) -> Option<MediaFrame> {
        let dropped = self.media_frames.pop_front();
        if let Some(frame) = dropped.as_ref() {
            if frame.is_audio() {
                self.audio_tag_cnt -= 1;
            } else if frame.is_video() {
                self.video_tag_cnt -= 1;
            }
        }
        self.first_video_dts_nano = self
            .media_frames
            .front()
            .map(|v| v.get_decode_timestamp_ns())
            .unwrap_or(0);
        self.last_video_dts_nano = self
            .media_frames
            .back()
            .map(|v| v.get_decode_timestamp_ns())
            .unwrap_or(0);

        dropped
    }

    #[inline]
    pub fn get_video_frame_cnt(&self) -> usize {
        self.video_tag_cnt
    }

    #[inline]
    pub fn get_audio_frame_cnt(&self) -> usize {
        self.audio_tag_cnt
    }

    #[inline]
    pub fn get_meta_frame_cnt(&self) -> usize {
        self.meta_tag_cnt
    }

    #[inline]
    pub fn get_first_video_dts_nano(&self) -> u64 {
        self.first_video_dts_nano
    }

    #[inline]
    pub fn get_last_video_dts_nano(&self) -> u64 {
        self.last_video_dts_nano
    }

    #[inline]
    pub fn get_last_video_frame_mut(&mut self) -> Option<&mut MediaFrame> {
        self.media_frames
            .iter_mut()
            .rev()
            .find(|frame| frame.is_video())
    }

    pub fn append_media_frame(&mut self, frame: MediaFrame) {
        match &frame {
            MediaFrame::VideoConfig { .. } => {
                self.video_tag_cnt += 1;
            }
            MediaFrame::Video { .. } => {
                self.video_tag_cnt += 1;
                if self.media_frames.is_empty() {
                    self.first_video_dts_nano = frame.get_decode_timestamp_ns();
                }
                self.last_video_dts_nano = frame.get_decode_timestamp_ns();
            }
            MediaFrame::Audio { .. } => self.audio_tag_cnt += 1,
            MediaFrame::AudioConfig { .. } => {
                self.audio_tag_cnt += 1;
            }
            MediaFrame::Script { .. } => self.meta_tag_cnt += 1,
        }

        self.media_frames.push_back(frame);
    }
}

impl Default for Gop {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct GopQueue {
    pub video_config: Option<VideoConfig>, // video config
    pub audio_config: Option<(AudioConfig, SoundInfoCommon)>, // audio config, sound info
    pub script_frame: Option<MediaFrame>,
    pub gops: VecDeque<Gop>,
    total_frame_cnt: u64,
    max_duration_ms: u64,
    max_frame_cnt: u64,
    dropped_gops_cnt: u64,
    dropped_video_cnt: u64,
    dropped_audio_cnt: u64,
}

impl GopQueue {
    pub fn new(max_duration_ms: u64, max_frame_cnt: u64) -> Self {
        Self {
            video_config: None,
            audio_config: None,
            script_frame: None,
            gops: VecDeque::new(),
            max_duration_ms,
            max_frame_cnt,
            total_frame_cnt: 0,
            dropped_gops_cnt: 0,
            dropped_video_cnt: 0,
            dropped_audio_cnt: 0,
        }
    }

    #[inline]
    pub fn get_gops_cnt(&self) -> usize {
        self.gops.len()
    }

    #[inline]
    pub fn get_dropped_gop_cnt(&self) -> u64 {
        self.dropped_gops_cnt
    }

    #[inline]
    pub fn get_dropped_video_cnt(&self) -> u64 {
        self.dropped_video_cnt
    }

    #[inline]
    pub fn get_dropped_audio_cnt(&self) -> u64 {
        self.dropped_audio_cnt
    }

    #[inline]
    fn accumulate_gops<'a, F>(&'a self, f: F) -> usize
    where
        F: Fn(&'a Gop) -> usize,
    {
        let mut result = 0;
        for gop in &self.gops {
            result += f(gop);
        }
        result
    }

    #[inline]
    pub fn get_video_frame_cnt(&self) -> usize {
        self.accumulate_gops(|gop| gop.get_video_frame_cnt())
    }

    #[inline]
    pub fn get_audio_frame_cut(&self) -> usize {
        self.accumulate_gops(|gop| gop.get_audio_frame_cnt())
    }

    #[inline]
    pub fn get_meta_frame_cnt(&self) -> usize {
        self.accumulate_gops(|gop| gop.get_meta_frame_cnt())
    }

    pub fn append_frame(&mut self, mut frame: MediaFrame) -> StreamCenterResult<()> {
        let span = tracing::trace_span!("gop cache append frame");
        let _enter = span.enter();

        if frame.is_video()
            && !frame.is_sequence_header()
            && !frame.is_video_key_frame()
            && self.get_video_frame_cnt() == 0
            && self.max_duration_ms != 0
            && self.max_frame_cnt != 0
        {
            tracing::warn!(
                "first video frame not key frame, dropping. frame codec id: {:?}, timestamp: {}",
                frame.video_codec_id(),
                frame.get_decode_timestamp_ns()
            );
            return Ok(());
        }
        let first_dts = self
            .gops
            .front()
            .map_or(0, |v| v.get_first_video_dts_nano());
        let last_dts = self.gops.back().map_or(0, |v| v.get_last_video_dts_nano());

        if (last_dts > first_dts
            && (last_dts - first_dts) >= self.max_duration_ms.checked_mul(1_000_000).unwrap())
            || self.total_frame_cnt >= self.max_frame_cnt
        {
            let span = tracing::trace_span!(
                "dopping gop",
                last_dts,
                first_dts,
                self.max_duration_ms,
                gops_cnt = self.gops.len(),
                self.total_frame_cnt,
                self.max_frame_cnt
            );
            let _enter = span.enter();
            let dropped = self.gops.pop_front();
            tracing::debug!(
                "dopping a whole gop, frame_cnt={}",
                dropped.as_ref().map_or(0, |v| v.get_meta_frame_cnt())
            );
            if let Some(gop) = dropped {
                self.dropped_gops_cnt += 1;
                self.dropped_video_cnt += gop.get_video_frame_cnt().to_u64().unwrap();
                self.dropped_audio_cnt += gop.get_audio_frame_cnt().to_u64().unwrap();
                self.total_frame_cnt -= gop.media_frames.len().to_u64().unwrap();
            }
        }

        let mut is_sequence_header = false;
        let mut is_video = false;
        match &mut frame {
            MediaFrame::Audio {
                frame_info: _,
                payload: _,
            } => {}
            MediaFrame::VideoConfig {
                timestamp_nano: _,
                config,
            } => {
                self.video_config = Some(*config.clone());
                is_sequence_header = true;
                tracing::info!("got video sh");
            }
            MediaFrame::AudioConfig {
                timestamp_nano: _,
                sound_info,
                config,
            } => {
                self.audio_config = Some((*config.clone(), *sound_info));
                is_sequence_header = true;
            }
            MediaFrame::Video {
                frame_info,
                payload: _,
            } => {
                is_video = true;
                if frame_info.frame_type == FrameType::KeyFrame {
                    self.gops.push_back(Gop::new());
                }
            }
            MediaFrame::Script {
                timestamp_nano: pts,
                on_meta_data,
                payload,
            } => {
                self.script_frame = Some(MediaFrame::Script {
                    timestamp_nano: *pts,
                    on_meta_data: on_meta_data.clone(),
                    payload: payload.clone(),
                });
                is_sequence_header = true;
                tracing::info!("meta, pts: {}, data: {:?}", pts, on_meta_data);
            }
        }

        if is_sequence_header {
            tracing::trace!("skip sequence header");
            return Ok(());
        }

        if self.gops.is_empty() && is_video {
            self.dropped_video_cnt += 1;
            return Ok(());
        }

        if self.gops.is_empty() {
            self.gops.push_back(Gop::new());
        }

        self.gops
            .back_mut()
            .expect("this cannot be empty")
            .append_media_frame(frame);
        self.total_frame_cnt += 1;

        Ok(())
    }
}
