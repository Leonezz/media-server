use crate::gop::MediaFrame;
use codec_common::{
    FrameType, MediaFrameTimestamp,
    audio::{AudioCodecCommon, AudioFrameInfo, SoundInfoCommon},
    video::{H264VideoConfig, VideoConfig, VideoFrameInfo, VideoFrameUnit},
};
use rtp_formats::packet::{
    packetizer::{
        RtpPacketizerAudioItem, RtpPacketizerItem, RtpPacketizerVideoItem,
        RtpTrivialPacketizerAACItem, RtpTrivialPacketizerH264Item,
    },
    sequencer::{RtpBufferAudioItem, RtpBufferItem, RtpBufferVideoItem},
};
use tokio_util::bytes::{BufMut, BytesMut};
use utils::traits::{dynamic_sized_packet::DynamicSizedPacket, writer::WriteTo};

pub fn from_media_frame(frame: MediaFrame) -> Option<RtpPacketizerItem> {
    match frame {
        MediaFrame::Video {
            frame_info: _,
            payload,
        } => match payload {
            VideoFrameUnit::H264 { nal_units } => Some(RtpPacketizerItem::Video(
                RtpPacketizerVideoItem::H264(RtpTrivialPacketizerH264Item { nalus: nal_units }),
            )),
            _ => unimplemented!("unsupported video format {:?}", payload),
        },
        MediaFrame::Audio {
            frame_info,
            payload,
        } => match frame_info.codec_id {
            AudioCodecCommon::AAC => Some(RtpPacketizerItem::Audio(RtpPacketizerAudioItem::AAC(
                RtpTrivialPacketizerAACItem {
                    access_units: vec![payload],
                },
            ))),
            _ => unimplemented!("unsupported audio format {:?}", frame_info),
        },
        MediaFrame::VideoConfig {
            timestamp_nano: _,
            config,
        } => match *config {
            VideoConfig::H264(H264VideoConfig {
                sps,
                pps,
                sps_ext: _,
                avc_decoder_configuration_record: _,
            }) => {
                let mut nal_units = Vec::new();
                if let Some(sps) = sps {
                    nal_units.push((&sps).into());
                }
                if let Some(pps) = pps {
                    nal_units.push((&pps).into());
                }
                Some(RtpPacketizerItem::Video(RtpPacketizerVideoItem::H264(
                    RtpTrivialPacketizerH264Item { nalus: nal_units },
                )))
            }
        },
        MediaFrame::AudioConfig {
            timestamp_nano: _,
            sound_info: _,
            config: _,
        } => {
            tracing::debug!("audio config frame, ignore");
            None
        }
        MediaFrame::Script {
            timestamp_nano: _,
            on_meta_data: _,
            payload: _,
        } => {
            tracing::debug!("script frame, ignore");
            None
        }
    }
}

pub fn to_media_frame(value: RtpBufferItem, timestamp_base: u32, clock_rate: u64) -> MediaFrame {
    let pts_nano = {
        let timestamp_diff = value
            .get_presentation_timestamp_ms()
            .wrapping_sub(timestamp_base) as u64;
        // Use 128-bit arithmetic to prevent overflow
        let nano_ticks = (timestamp_diff as u128) * 1_000_000_000u128;
        let result = nano_ticks / (clock_rate as u128);
        result as u64 // Safe because result will be much smaller than u64::MAX
    };
    match value {
        RtpBufferItem::Audio(audio) => match audio {
            RtpBufferAudioItem::AAC(aac) => {
                let mut bytes = BytesMut::zeroed(aac.access_unit.get_packet_bytes_count());
                aac.access_unit
                    .write_to(&mut bytes.as_mut().writer())
                    .unwrap();
                MediaFrame::Audio {
                    frame_info: AudioFrameInfo {
                        codec_id: AudioCodecCommon::AAC,
                        frame_type: FrameType::CodedFrames,
                        sound_info: SoundInfoCommon {
                            sound_rate: codec_common::audio::SoundRateCommon::KHZ44,
                            sound_size: codec_common::audio::SoundSizeCommon::Bit16,
                            sound_type: codec_common::audio::SoundTypeCommon::Stereo,
                        },
                        timestamp_nano: pts_nano,
                    },
                    payload: bytes.freeze(),
                }
            }
        },
        RtpBufferItem::Video(video) => match video {
            RtpBufferVideoItem::H264(h264) => {
                let is_idr = h264.is_idr;
                let mut nal_units = vec![];
                if is_idr {
                    if let Some(sps) = h264.sps {
                        nal_units.push(sps);
                    }
                    if let Some(pps) = h264.pps {
                        nal_units.push(pps);
                    }
                }
                nal_units.extend(h264.nal_units);
                MediaFrame::Video {
                    frame_info: VideoFrameInfo {
                        codec_id: codec_common::video::VideoCodecCommon::AVC,
                        frame_type: if is_idr {
                            FrameType::KeyFrame
                        } else {
                            FrameType::CodedFrames
                        },
                        timestamp: MediaFrameTimestamp::with_timestamp_nano(pts_nano),
                    },
                    payload: codec_common::video::VideoFrameUnit::H264 { nal_units },
                }
            }
        },
    }
}
