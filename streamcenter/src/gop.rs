use std::{
    collections::{HashMap, VecDeque},
    io,
};

use bitstream_io::{BitRead, BitWrite};
use codec_aac::mpeg4_configuration::audio_specific_config::AudioSpecificConfig;
use codec_common::{
    FrameType,
    audio::{AudioCodecCommon, AudioConfig, AudioFrameInfo, SoundInfoCommon},
    video::{VideoCodecCommon, VideoConfig, VideoFrameInfo, VideoFrameUnit},
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
use tokio_util::bytes::{Buf, Bytes, BytesMut};
use utils::traits::reader::ReadFrom;
use utils::traits::writer::{BitwiseWriteTo, WriteTo};
use utils::traits::{
    dynamic_sized_packet::{DynamicSizedBitsPacket, DynamicSizedPacket},
    reader::BitwiseReadFrom,
};

use crate::errors::{StreamCenterError, StreamCenterResult};

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

    pub fn get_timestamp_ns(&self) -> u64 {
        match self {
            Self::Audio {
                frame_info,
                payload: _,
            } => frame_info.timestamp_nano,
            Self::AudioConfig {
                timestamp_nano,
                sound_info: _,
                config: _,
            } => *timestamp_nano,
            Self::Script {
                timestamp_nano,
                on_meta_data: _,
                payload: _,
            } => *timestamp_nano,
            Self::Video {
                frame_info,
                payload: _,
            } => frame_info.timestamp_nano,
            Self::VideoConfig {
                timestamp_nano,
                config: _,
            } => *timestamp_nano,
        }
    }

    pub fn get_timestamp_ns_mut(&mut self) -> &mut u64 {
        match self {
            Self::Audio {
                frame_info,
                payload: _,
            } => &mut frame_info.timestamp_nano,
            Self::AudioConfig {
                timestamp_nano,
                sound_info: _,
                config: _,
            } => timestamp_nano,
            Self::Script {
                timestamp_nano,
                on_meta_data: _,
                payload: _,
            } => timestamp_nano,
            Self::Video {
                frame_info,
                payload: _,
            } => &mut frame_info.timestamp_nano,
            Self::VideoConfig {
                timestamp_nano,
                config: _,
            } => timestamp_nano,
        }
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
        match self {
            Self::Audio {
                frame_info,
                payload,
            } => {
                let legacy_header: LegacyAudioTagHeader = frame_info.try_into()?;
                Ok(flv_formats::tag::FLVTag {
                    tag_header: flv_formats::tag::flv_tag_header::FLVTagHeader {
                        tag_type: FLVTagType::Audio,
                        data_size: legacy_header
                            .get_packet_bytes_count()
                            .checked_add(payload.len())
                            .and_then(|v| v.to_u32())
                            .unwrap(),
                        timestamp: frame_info
                            .timestamp_nano
                            .checked_div(1_000_000)
                            .and_then(|v| v.to_u32())
                            .unwrap(),
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
            Self::Script {
                timestamp_nano,
                on_meta_data,
                payload: _,
            } => {
                let value = vec![
                    amf_formats::amf0::string("@setDataFrame"),
                    amf_formats::amf0::string("@onMetaData"),
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
                        timestamp: timestamp_nano
                            .checked_div(1_000_000)
                            .and_then(|v| v.to_u32())
                            .unwrap(),
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
                let legacy_header: LegacyVideoTagHeader = frame_info.try_into()?;
                let mut bytes =
                    BytesMut::zeroed(payload.bytes_cnt(nalu_size_length.to_usize().unwrap()));
                let mut writer = io::Cursor::new(bytes.as_mut());
                codec_common::video::writer::VideoFrameUnitAvccWriter(payload, nalu_size_length)
                    .write_to(&mut writer)
                    .map_err(|err| {
                        StreamCenterError::RemuxFailed(format!(
                            "remux from video frame to flv video tag failed: {}",
                            err
                        ))
                    })?;

                Ok(flv_formats::tag::FLVTag {
                    tag_header: flv_formats::tag::flv_tag_header::FLVTagHeader {
                        tag_type: FLVTagType::Video,
                        data_size: legacy_header
                            .get_packet_bytes_count()
                            .checked_add(bytes.len())
                            .and_then(|v| v.to_u32())
                            .unwrap(),
                        timestamp: frame_info
                            .timestamp_nano
                            .checked_div(1_000_000)
                            .and_then(|v| v.to_u32())
                            .unwrap(),
                        filter_enabled: false,
                    },
                    body_with_filter: flv_formats::tag::flv_tag_body::FLVTagBodyWithFilter {
                        filter: None,
                        body: flv_formats::tag::flv_tag_body::FLVTagBody::Video {
                            header: flv_formats::tag::video_tag_header::VideoTagHeader::Legacy(
                                legacy_header,
                            ),
                            body: bytes.freeze(),
                        },
                    },
                })
            }
            Self::VideoConfig {
                timestamp_nano,
                config,
            } => {
                let frame_info = VideoFrameInfo {
                    codec_id: config.as_ref().into(),
                    frame_type: FrameType::SequenceStart,
                    timestamp_nano: *timestamp_nano,
                };
                let legacy_header: LegacyVideoTagHeader = (&frame_info).try_into()?;
                match config.as_ref() {
                    VideoConfig::H264 {
                        sps: _,
                        pps: _,
                        sps_ext: _,
                        avc_decoder_configuration_record,
                    } => {
                        if let Some(record) = avc_decoder_configuration_record {
                            let mut bytes = BytesMut::zeroed(record.get_packet_bytes_count());
                            let mut writer = io::Cursor::new(bytes.as_mut());
                            record.write_to(&mut writer)?;
                            Ok(flv_formats::tag::FLVTag {
                                tag_header: flv_formats::tag::flv_tag_header::FLVTagHeader {
                                    tag_type: FLVTagType::Video,
                                    data_size: legacy_header
                                        .get_packet_bytes_count()
                                        .checked_add(bytes.len())
                                        .and_then(|v| v.to_u32())
                                        .unwrap(),
                                    timestamp: timestamp_nano
                                        .checked_div(1_000_000)
                                        .and_then(|v| v.to_u32())
                                        .unwrap(),
                                    filter_enabled: false,
                                },
                                body_with_filter:
                                    flv_formats::tag::flv_tag_body::FLVTagBodyWithFilter {
                                        filter: None,
                                        body: flv_formats::tag::flv_tag_body::FLVTagBody::Video {
                                            header: flv_formats::tag::video_tag_header::VideoTagHeader::Legacy(legacy_header),
                                            body: bytes.freeze(),
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
                let legacy_header: LegacyAudioTagHeader = (&frame_info).try_into()?;
                match config.as_ref() {
                    AudioConfig::AAC(config) => {
                        let mut bytes = BytesMut::zeroed(
                            config
                                .get_packet_bits_count()
                                .checked_add(4)
                                .and_then(|v| v.checked_div(8))
                                .unwrap(),
                        );
                        let mut writer = bitstream_io::BitWriter::endian(
                            bytes.as_mut(),
                            bitstream_io::BigEndian,
                        );
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
                                timestamp: timestamp_nano
                                    .checked_div(1_000_000)
                                    .and_then(|v| v.to_u32())
                                    .unwrap(),
                                filter_enabled: false,
                            },
                            body_with_filter:
                                flv_formats::tag::flv_tag_body::FLVTagBodyWithFilter {
                                    filter: None,
                                    body: flv_formats::tag::flv_tag_body::FLVTagBody::Audio {
                                        header: flv_formats::tag::audio_tag_header::AudioTagHeader::Legacy(legacy_header),
                                        body: bytes.freeze(),
                                    },
                                },
                        })
                    }
                }
            }
        }
    }

    pub fn from_flv_tag(tag: FLVTag, nalu_size_length: u8) -> StreamCenterResult<Self> {
        match tag.body_with_filter.body {
            FLVTagBody::Audio { header, body } => {
                let tag_header_info: AudioTagHeaderWithoutMultiTrack = (&header).try_into()?;
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
                        Ok(Self::Video {
                            frame_info: VideoFrameInfo::new(
                                tag_header_info.codec_id,
                                if tag_header_info.packet_type == VideoPacketType::SequenceEnd {
                                    FrameType::SequenceEnd
                                } else if tag_header_info.frame_type == FrameTypeFLV::KeyFrame {
                                    FrameType::KeyFrame
                                } else {
                                    FrameType::CodedFrames
                                },
                                tag.tag_header
                                    .timestamp
                                    .to_u64()
                                    .and_then(|v| v.checked_mul(1_000_000))
                                    .and_then(|v| {
                                        v.checked_add(
                                            tag_header_info
                                                .composition_time
                                                .unwrap_or(0)
                                                .to_u64()
                                                .and_then(|v| v.checked_mul(1_000_000))
                                                .unwrap(),
                                        )
                                    })
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
    first_video_pts_nano: u64,
    last_video_pts_nano: u64,
}

impl Gop {
    pub fn new() -> Self {
        Self {
            media_frames: VecDeque::new(),
            video_tag_cnt: 0,
            audio_tag_cnt: 0,
            meta_tag_cnt: 0,
            first_video_pts_nano: 0,
            last_video_pts_nano: 0,
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
        self.first_video_pts_nano = self
            .media_frames
            .front()
            .map(|v| v.get_timestamp_ns())
            .unwrap_or(0);
        self.last_video_pts_nano = self
            .media_frames
            .back()
            .map(|v| v.get_timestamp_ns())
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
    pub fn get_first_video_timestamp_nano(&self) -> u64 {
        self.first_video_pts_nano
    }

    #[inline]
    pub fn get_last_video_timestamp_nano(&self) -> u64 {
        self.last_video_pts_nano
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
            MediaFrame::VideoConfig {
                timestamp_nano: _,
                config: _,
            } => {
                self.video_tag_cnt += 1;
            }
            MediaFrame::Video {
                frame_info,
                payload: _,
            } => {
                self.video_tag_cnt += 1;
                if self.media_frames.is_empty() {
                    self.first_video_pts_nano = frame_info.timestamp_nano;
                }
                self.last_video_pts_nano = frame_info.timestamp_nano;
            }
            MediaFrame::Audio {
                frame_info: _,
                payload: _,
            } => self.audio_tag_cnt += 1,
            MediaFrame::AudioConfig { .. } => {
                self.audio_tag_cnt += 1;
            }
            MediaFrame::Script {
                timestamp_nano: _,
                payload: _,
                on_meta_data: _,
            } => self.meta_tag_cnt += 1,
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
    pub video_config: Option<VideoConfig>,
    pub audio_config: Option<(AudioConfig, SoundInfoCommon)>,
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
                tracing::info!("got audio sh");
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
                tracing::info!("meta, pts: {}, data: {:?}", pts, on_meta_data);
            }
        }

        if self.gops.is_empty() && is_video {
            self.dropped_video_cnt += 1;
            return Ok(());
        }

        if self.gops.is_empty() {
            self.gops.push_back(Gop::new());
        }

        let first_pts = self
            .gops
            .front()
            .expect("this cannot be empty")
            .get_first_video_timestamp_nano();
        let last_pts = self
            .gops
            .back()
            .expect("this cannot be empty")
            .get_last_video_timestamp_nano();

        if (last_pts > first_pts && (last_pts - first_pts) >= self.max_duration_ms)
            || self.total_frame_cnt >= self.max_frame_cnt
        {
            if self.gops.len() > 1 {
                let dropped = self.gops.pop_front();
                if let Some(gop) = dropped {
                    self.dropped_gops_cnt += 1;
                    self.dropped_video_cnt += gop.get_video_frame_cnt() as u64;
                    self.dropped_audio_cnt += gop.get_audio_frame_cnt() as u64;
                }
            } else if self.gops.len() == 1 {
                let dropped = self.gops[0].pop_front();
                if let Some(dropped) = dropped {
                    if dropped.is_video() {
                        self.dropped_video_cnt += 1;
                    } else if dropped.is_audio() {
                        self.dropped_audio_cnt += 1;
                    }
                }
            }
        }

        if !is_sequence_header {
            self.gops
                .back_mut()
                .expect("this cannot be empty")
                .append_media_frame(frame);
            self.total_frame_cnt += 1;
        }

        Ok(())
    }
}
