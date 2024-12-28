use std::collections::HashMap;

use tokio_util::either::Either;

use crate::errors::FLVError;

use super::{
    enhanced::{
        AvMultiTrackType,
        ex_video::ex_video_header::{
            ExVideoTagHeader, VideoFourCC, VideoModEx, VideoPacketType, VideoTrackInfo,
        },
    },
    video_tag_header::{self, FrameType, VideoCommand},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoCodecCommon {
    SorensonH263,
    ScreenVideo,
    On2VP6,
    On2VP6WithAlpha,
    ScreenVideoV2,
    AVC,
    HEVC,
    VP8,
    VP9,
    AV1,
}

impl TryInto<video_tag_header::CodecID> for VideoCodecCommon {
    type Error = FLVError;
    fn try_into(self) -> Result<video_tag_header::CodecID, Self::Error> {
        match self {
            Self::SorensonH263 => Ok(video_tag_header::CodecID::SorensonH263),
            Self::ScreenVideo => Ok(video_tag_header::CodecID::ScreenVideo),
            Self::On2VP6 => Ok(video_tag_header::CodecID::On2VP6),
            Self::On2VP6WithAlpha => Ok(video_tag_header::CodecID::On2VP6WithAlpha),
            Self::ScreenVideoV2 => Ok(video_tag_header::CodecID::ScreenVideoV2),
            Self::AVC => Ok(video_tag_header::CodecID::AVC),
            Self::HEVC => Ok(video_tag_header::CodecID::HEVC),
            _ => Err(FLVError::UnknownCodecID(255)),
        }
    }
}

impl From<video_tag_header::CodecID> for VideoCodecCommon {
    fn from(value: video_tag_header::CodecID) -> Self {
        match value {
            video_tag_header::CodecID::SorensonH263 => Self::SorensonH263,
            video_tag_header::CodecID::ScreenVideo => Self::ScreenVideo,
            video_tag_header::CodecID::On2VP6 => Self::On2VP6,
            video_tag_header::CodecID::On2VP6WithAlpha => Self::On2VP6WithAlpha,
            video_tag_header::CodecID::ScreenVideoV2 => Self::ScreenVideoV2,
            video_tag_header::CodecID::AVC => Self::AVC,
            video_tag_header::CodecID::HEVC => Self::HEVC,
            video_tag_header::CodecID::AV1 => Self::AV1,
        }
    }
}

impl TryInto<VideoFourCC> for VideoCodecCommon {
    type Error = FLVError;
    fn try_into(self) -> Result<VideoFourCC, Self::Error> {
        match self {
            Self::AVC => Ok(VideoFourCC::AVC),
            Self::HEVC => Ok(VideoFourCC::HEVC),
            Self::VP8 => Ok(VideoFourCC::VP8),
            Self::VP9 => Ok(VideoFourCC::VP9),
            Self::AV1 => Ok(VideoFourCC::AV1),
            _ => Err(FLVError::UnknownFourCC(format!(
                "trying to convert unaligned legacy flv video codec: {:?} to VideoFourCC",
                self
            ))),
        }
    }
}

impl From<VideoFourCC> for VideoCodecCommon {
    fn from(value: VideoFourCC) -> Self {
        match value {
            VideoFourCC::AV1 => Self::AV1,
            VideoFourCC::AVC => Self::AVC,
            VideoFourCC::HEVC => Self::HEVC,
            VideoFourCC::VP8 => Self::VP8,
            VideoFourCC::VP9 => Self::VP9,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VideoTagHeaderWithoutMultiTrack {
    pub packet_type: VideoPacketType,
    pub codec_id: VideoCodecCommon,
    pub frame_type: FrameType,
    pub video_command: Option<VideoCommand>,
    pub composition_time: Option<u32>,
    pub timestamp_nano: Option<u32>,
    pub track_type: Option<AvMultiTrackType>,
    // for debug
    pub is_enhanced_rtmp: bool,
}

impl TryInto<video_tag_header::VideoTagHeader> for VideoTagHeaderWithoutMultiTrack {
    type Error = FLVError;
    fn try_into(self) -> Result<video_tag_header::VideoTagHeader, Self::Error> {
        let codec_id: video_tag_header::CodecID = self.codec_id.try_into()?;
        let mut avc_packet_type = None;
        if self.codec_id == VideoCodecCommon::AVC
            || self.codec_id == VideoCodecCommon::HEVC
            || self.codec_id == VideoCodecCommon::AV1
        {
            match self.packet_type {
                VideoPacketType::SequenceStart => {
                    avc_packet_type = Some(video_tag_header::AVCPacketType::SequenceHeader);
                }
                VideoPacketType::CodedFrames | VideoPacketType::CodedFramesX => {
                    avc_packet_type = Some(video_tag_header::AVCPacketType::NALU);
                }
                VideoPacketType::SequenceEnd => {
                    avc_packet_type = Some(video_tag_header::AVCPacketType::EndOfSequence);
                }
                _ => {}
            }
        };
        Ok(video_tag_header::VideoTagHeader {
            frame_type: self.frame_type,
            codec_id,
            avc_packet_type,
            video_command: self.video_command,
            composition_time: self.composition_time,
        })
    }
}

impl TryInto<ExVideoTagHeader> for VideoTagHeaderWithoutMultiTrack {
    type Error = FLVError;
    fn try_into(self) -> Result<ExVideoTagHeader, Self::Error> {
        let mut tracks: HashMap<u8, VideoTrackInfo> = HashMap::new();
        tracks.insert(0, VideoTrackInfo {
            codec: self.codec_id.try_into()?,
            composition_time: self.composition_time,
        });

        Ok(ExVideoTagHeader {
            packet_type: self.packet_type,
            frame_type: self.frame_type,
            packet_mod_ex: VideoModEx {
                timestamp_nano: self.timestamp_nano,
            },
            track_type: self.track_type,
            video_command: self.video_command,
            tracks,
        })
    }
}

impl From<video_tag_header::VideoTagHeader> for VideoTagHeaderWithoutMultiTrack {
    fn from(value: video_tag_header::VideoTagHeader) -> Self {
        let packet_type = if let Some(packet_type) = value.avc_packet_type {
            packet_type.into()
        } else {
            VideoPacketType::CodedFrames
        };

        Self {
            packet_type,
            codec_id: value.codec_id.into(),
            frame_type: value.frame_type,
            video_command: value.video_command,
            timestamp_nano: None,
            track_type: None,
            composition_time: value.composition_time,
            is_enhanced_rtmp: false,
        }
    }
}

impl TryFrom<ExVideoTagHeader> for VideoTagHeaderWithoutMultiTrack {
    type Error = FLVError;
    fn try_from(value: ExVideoTagHeader) -> Result<Self, Self::Error> {
        let track_info = value.tracks.get(&0);
        if track_info.is_none() {
            return Err(FLVError::InconsistentHeader(format!(
                "expect a valid ExVideoHeader, got {:?} instead",
                value
            )));
        }
        let track_info = track_info.unwrap();
        Ok(Self {
            packet_type: value.packet_type,
            codec_id: track_info.codec.into(),
            frame_type: value.frame_type,
            video_command: value.video_command,
            composition_time: track_info.composition_time,
            timestamp_nano: value.packet_mod_ex.timestamp_nano,
            track_type: value.track_type,
            is_enhanced_rtmp: true,
        })
    }
}

impl VideoTagHeaderWithoutMultiTrack {
    pub fn is_sequence_header(&self) -> bool {
        match self.packet_type {
            VideoPacketType::SequenceStart => true,
            _ => false,
        }
    }

    pub fn is_key_frame(&self) -> bool {
        match self.frame_type {
            FrameType::KeyFrame => true,
            _ => false,
        }
    }

    pub fn get_codec_id(&self) -> VideoCodecCommon {
        self.codec_id
    }
}

impl TryFrom<Either<video_tag_header::VideoTagHeader, ExVideoTagHeader>>
    for VideoTagHeaderWithoutMultiTrack
{
    type Error = FLVError;
    fn try_from(
        value: Either<video_tag_header::VideoTagHeader, ExVideoTagHeader>,
    ) -> Result<Self, Self::Error> {
        match value {
            Either::Left(header) => Ok(header.into()),
            Either::Right(header) => Ok(header.try_into()?),
        }
    }
}
