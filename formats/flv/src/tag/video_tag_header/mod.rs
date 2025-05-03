use codec_common::{
    FrameType,
    video::{VideoCodecCommon, VideoFrameInfo},
};
use utils::traits::dynamic_sized_packet::DynamicSizedPacket;

use crate::errors::FLVError;

use super::enhanced::ex_video::ex_video_header::ExVideoTagHeader;

pub mod reader;
pub mod writer;
///
/// Type of video frame.
/// The following values are defined:
/// 1 = key frame (for AVC, a seekable frame)
/// 2 = inter frame (for AVC, a non-seekable frame)
/// 3 = disposable inter frame (H.263 only)
/// 4 = generated key frame (reserved for server use only)
/// 5 = video info/command frame
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub enum FrameTypeFLV {
    #[default]
    KeyFrame = 1,
    InterFrame = 2,
    DisposableInterFrame = 3,
    GeneratedKeyFrame = 4,
    CommandFrame = 5,
}

impl From<FrameType> for FrameTypeFLV {
    fn from(value: FrameType) -> Self {
        match value {
            FrameType::CodedFrames => Self::InterFrame,
            FrameType::KeyFrame => Self::KeyFrame,
            FrameType::SequenceEnd => Self::InterFrame,
            FrameType::SequenceStart => Self::InterFrame,
        }
    }
}

impl From<FrameTypeFLV> for u8 {
    fn from(value: FrameTypeFLV) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for FrameTypeFLV {
    type Error = FLVError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::KeyFrame),
            2 => Ok(Self::InterFrame),
            3 => Ok(Self::DisposableInterFrame),
            4 => Ok(Self::GeneratedKeyFrame),
            5 => Ok(Self::CommandFrame),
            _ => Err(FLVError::UnknownVideoFrameType(value)),
        }
    }
}

///
/// Codec Identifier.
/// The following values are defined:
/// 2 = Sorenson H.263
/// 3 = Screen video
/// 4 = On2 VP6
/// 5 = On2 VP6 with alpha channel
/// 6 = Screen video version 2
/// 7 = AVC
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub enum CodecID {
    SorensonH263 = 2,
    ScreenVideo = 3,
    On2VP6 = 4,
    On2VP6WithAlpha = 5,
    ScreenVideoV2 = 6,
    #[default]
    AVC = 7,
    // not standard, but used a lot in china, @see: https://github.com/CDN-Union/H265
    HEVC = 12,
    // not standard, but used a lot in china, @see: https://mp.weixin.qq.com/s/H3qI7zsON5sdf4oDJ9qlkg
    AV1 = 13,
}

impl From<CodecID> for u8 {
    fn from(value: CodecID) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for CodecID {
    type Error = FLVError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            2 => Ok(Self::SorensonH263),
            3 => Ok(Self::ScreenVideo),
            4 => Ok(Self::On2VP6),
            5 => Ok(Self::On2VP6WithAlpha),
            6 => Ok(Self::ScreenVideoV2),
            7 => Ok(Self::AVC),
            12 => Ok(Self::HEVC),
            13 => Ok(Self::AV1),
            _ => Err(FLVError::UnknownCodecID(value)),
        }
    }
}

///
/// IF CodecID == 7
/// The following values are defined:
/// 0 = AVC sequence header
/// 1 = AVC NALU
/// 2 = AVC end of sequence (lower level NALU sequence ender is not required or supported)
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AVCPacketType {
    SequenceHeader = 0,
    NALU = 1,
    EndOfSequence = 2,
}

impl From<AVCPacketType> for u8 {
    fn from(value: AVCPacketType) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for AVCPacketType {
    type Error = FLVError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::SequenceHeader),
            1 => Ok(Self::NALU),
            2 => Ok(Self::EndOfSequence),
            _ => Err(FLVError::UnknownAVCPacketType(value)),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoCommand {
    StartSeek = 0,
    EndSeek = 1,
    // 0x03 = reserved
    // ...
    // 0xff = reserved
}

impl From<VideoCommand> for u8 {
    fn from(value: VideoCommand) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for VideoCommand {
    type Error = FLVError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(VideoCommand::StartSeek),
            1 => Ok(VideoCommand::EndSeek),
            _ => Err(FLVError::UnknownVideoCommandType(value)),
        }
    }
}
#[derive(Debug, Clone, Copy, Default)]
pub struct LegacyVideoTagHeader {
    pub frame_type: FrameTypeFLV,
    pub codec_id: CodecID,
    pub avc_packet_type: Option<AVCPacketType>,
    pub video_command: Option<VideoCommand>,
    ///
    /// IF CodecID == 7
    /// IF AVCPacketType == 1
    ///   Composition time offset
    /// ELSE 0
    /// See ISO 14496-12, 8.15.3 for an explanation of composition times.
    /// The offset in an FLV file is always in milliseconds.
    pub composition_time: Option<u32>,
}

impl DynamicSizedPacket for LegacyVideoTagHeader {
    fn get_packet_bytes_count(&self) -> usize {
        let mut result = 1;
        if self.frame_type == FrameTypeFLV::CommandFrame && self.video_command.is_some() {
            result += 1;
        }
        if self.codec_id == CodecID::AVC
            || self.codec_id == CodecID::HEVC
            || self.codec_id == CodecID::AV1
        {
            result += 4
        }
        result
    }
}

impl TryFrom<&VideoFrameInfo> for LegacyVideoTagHeader {
    type Error = FLVError;
    fn try_from(value: &VideoFrameInfo) -> Result<Self, Self::Error> {
        let packet_type = if value.codec_id == VideoCodecCommon::AVC
            || value.codec_id == VideoCodecCommon::HEVC
            || value.codec_id == VideoCodecCommon::AV1
        {
            match value.frame_type {
                FrameType::CodedFrames => Some(AVCPacketType::NALU),
                FrameType::KeyFrame => Some(AVCPacketType::NALU),
                FrameType::SequenceEnd => Some(AVCPacketType::EndOfSequence),
                FrameType::SequenceStart => Some(AVCPacketType::SequenceHeader),
            }
        } else {
            None
        };
        Ok(Self {
            frame_type: value.frame_type.into(),
            codec_id: value.codec_id.try_into()?,
            avc_packet_type: packet_type,
            video_command: None,
            composition_time: Some(0),
        })
    }
}

impl LegacyVideoTagHeader {
    #[inline]
    pub fn get_frame_type(&self) -> FrameTypeFLV {
        self.frame_type
    }

    #[inline]
    pub fn get_codec_id(&self) -> CodecID {
        self.codec_id
    }

    #[inline]
    pub fn get_packet_type(&self) -> Option<AVCPacketType> {
        self.avc_packet_type
    }

    #[inline]
    pub fn is_key_frame(&self) -> bool {
        self.frame_type == FrameTypeFLV::KeyFrame
    }

    #[inline]
    pub fn is_sequence_header(&self) -> bool {
        match self.avc_packet_type {
            None => false,
            Some(packet_type) => packet_type == AVCPacketType::SequenceHeader,
        }
    }

    #[inline]
    pub fn is_avc_nalu(&self) -> bool {
        match self.avc_packet_type {
            None => false,
            Some(packet_type) => packet_type == AVCPacketType::NALU,
        }
    }
}

#[derive(Debug)]
pub enum VideoTagHeader {
    Legacy(LegacyVideoTagHeader),
    Enhanced(ExVideoTagHeader),
}
