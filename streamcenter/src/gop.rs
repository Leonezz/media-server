use std::{
    collections::{HashMap, VecDeque},
    io,
};

use codec_common::{
    FrameType,
    audio::AudioFrameInfo,
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
use utils::traits::dynamic_sized_packet::DynamicSizedPacket;
use utils::traits::reader::ReadFrom;
use utils::traits::writer::WriteTo;

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
            }
        )
    }

    #[inline]
    pub fn is_audio(&self) -> bool {
        matches!(
            self,
            MediaFrame::Audio {
                frame_info: _,
                payload: _,
            }
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

    #[inline]
    pub fn is_sequence_header(&self) -> bool {
        match self {
            MediaFrame::Audio {
                frame_info,
                payload: _,
            } => frame_info.frame_type == FrameType::SequenceStart,
            MediaFrame::Video {
                frame_info,
                payload: _,
            } => frame_info.frame_type == FrameType::SequenceStart,
            _ => false,
        }
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
                payload,
            } => Ok(flv_formats::tag::FLVTag {
                tag_header: flv_formats::tag::flv_tag_header::FLVTagHeader {
                    tag_type: FLVTagType::Script,
                    data_size: payload.len().to_u32().unwrap(),
                    timestamp: timestamp_nano
                        .checked_div(1_000_000)
                        .and_then(|v| v.to_u32())
                        .unwrap(),
                    filter_enabled: false,
                },
                body_with_filter: flv_formats::tag::flv_tag_body::FLVTagBodyWithFilter {
                    filter: None,
                    body: flv_formats::tag::flv_tag_body::FLVTagBody::Script {
                        value: vec![
                            amf_formats::amf0::string("@setDataFrame"),
                            amf_formats::amf0::string("@onMetaData"),
                            amf_formats::amf0::Value::ECMAArray(
                                on_meta_data.clone().map_or(vec![], |ref v| v.into()),
                            ),
                        ],
                    },
                },
            }),
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
        }
    }

    pub fn from_flv_tag(tag: FLVTag, nalu_size_length: u8) -> StreamCenterResult<Self> {
        match tag.body_with_filter.body {
            FLVTagBody::Audio { header, body } => {
                let tag_header_info: AudioTagHeaderWithoutMultiTrack = (&header).try_into()?;
                Ok(Self::Audio {
                    frame_info: AudioFrameInfo::new(
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
                    ),
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
                                tracing::debug!(
                                    "got avc_decoder_configuration_record: {:?}",
                                    config
                                );
                                VideoConfig::from(config)
                            }
                            _ => {
                                todo!()
                            }
                        };
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
    pub flv_frames: Vec<MediaFrame>,
    video_tag_cnt: usize,
    audio_tag_cnt: usize,
    meta_tag_cnt: usize,
    first_video_pts_nano: u64,
    last_video_pts_nano: u64,
}

impl Gop {
    pub fn new() -> Self {
        Self {
            flv_frames: Vec::new(),
            video_tag_cnt: 0,
            audio_tag_cnt: 0,
            meta_tag_cnt: 0,
            first_video_pts_nano: 0,
            last_video_pts_nano: 0,
        }
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
    pub fn get_first_video_pts(&self) -> u64 {
        self.first_video_pts_nano
    }

    #[inline]
    pub fn get_last_video_pts(&self) -> u64 {
        self.last_video_pts_nano
    }

    pub fn append_media_frame(&mut self, frame: MediaFrame) {
        match &frame {
            MediaFrame::VideoConfig {
                timestamp_nano: _,
                config: _,
            } => {}
            MediaFrame::Video {
                frame_info,
                payload: _,
            } => {
                self.video_tag_cnt += 1;
                if self.flv_frames.is_empty() {
                    self.first_video_pts_nano = frame_info.timestamp_nano;
                }
                self.last_video_pts_nano = frame_info.timestamp_nano;
            }
            MediaFrame::Audio {
                frame_info: _,
                payload: _,
            } => self.audio_tag_cnt += 1,
            MediaFrame::Script {
                timestamp_nano: _,
                payload: _,
                on_meta_data: _,
            } => self.meta_tag_cnt += 1,
        }

        self.flv_frames.push(frame);
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
    pub audio_config: Option<MediaFrame>,
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

    pub fn append_frame(&mut self, frame: MediaFrame) -> StreamCenterResult<()> {
        let mut is_sequence_header = false;
        let mut is_video = false;
        match &frame {
            MediaFrame::Audio {
                frame_info,
                payload: _,
            } => {
                if frame_info.frame_type == FrameType::SequenceStart {
                    tracing::info!("audio header: {:?}", frame_info);
                    self.audio_config = Some(frame.clone());
                    is_sequence_header = true;
                }
            }
            MediaFrame::VideoConfig {
                timestamp_nano: _,
                config,
            } => {
                self.video_config = Some(*config.clone());
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
                payload: _,
            } => {
                self.script_frame = Some(frame.clone());
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
            .get_first_video_pts();
        let last_pts = self
            .gops
            .back()
            .expect("this cannot be empty")
            .get_last_video_pts();

        if ((last_pts > first_pts && (last_pts - first_pts) >= self.max_duration_ms)
            || self.total_frame_cnt >= self.max_frame_cnt)
            && self.gops.len() > 1
        {
            let dropped = self.gops.pop_front();
            if let Some(gop) = dropped {
                self.dropped_gops_cnt += 1;
                self.dropped_video_cnt += gop.get_video_frame_cnt() as u64;
                self.dropped_audio_cnt += gop.get_audio_frame_cnt() as u64;
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
