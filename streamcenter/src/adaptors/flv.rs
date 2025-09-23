use crate::{
    errors::{StreamCenterError, StreamCenterResult},
    gop::MediaFrame,
};
use bitstream_io::{BitRead, BitWrite};
use codec_aac::mpeg4_configuration::audio_specific_config::AudioSpecificConfig;
use codec_common::{
    FrameType, MediaFrameTimestamp,
    audio::{AudioConfig, AudioFrameInfo},
    video::{H264VideoConfig, VideoCodecCommon, VideoConfig, VideoFrameInfo},
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
use std::{collections::HashMap, io};
use tokio_util::bytes::{Buf, Bytes};
use tracing::debug_span;
use utils::traits::{
    dynamic_sized_packet::{DynamicSizedBitsPacket, DynamicSizedPacket},
    reader::{BitwiseReadFrom, ReadFrom},
    writer::{BitwiseWriteTo, WriteTo},
};

impl MediaFrame {
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
