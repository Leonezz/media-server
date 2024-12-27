use std::collections::HashMap;

use crate::{
    errors::FLVError,
    tag::{
        enhanced::AvMultiTrackType,
        video_tag_header::{AVCPacketType, FrameType, VideoCommand},
    },
};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoPacketType {
    SequenceStart = 0,
    CodedFrames = 1,
    SequenceEnd = 2,

    // CompositionTime Offset is implicitly set to zero. This optimization
    // avoids transmitting an SI24 composition time value of zero over the wire.
    // See the ExVideoTagBody section below for corresponding pseudocode.
    CodedFramesX = 3,

    // ExVideoTagBody does not contain video data. Instead, it contains
    // an AMF-encoded metadata. Refer to the Metadata Frame section for
    // an illustration of its usage. For example, the metadata might include
    // HDR information. This also enables future possibilities for expressing
    // additional metadata meant for subsequent video sequences.
    //
    // If VideoPacketType.Metadata is present, the FrameType flags
    // at the top of this table should be ignored.
    Metadata = 4,

    // Carriage of bitstream in MPEG-2 TS format
    //
    // PacketTypeSequenceStart and PacketTypeMPEG2TSSequenceStart
    // are mutually exclusive
    MPEG2TSSequenceStart = 5,

    // Turns on video multitrack mode
    Multitrack = 6,

    // ModEx is a special signal within the VideoPacketType enum that
    // serves to both modify and extend the behavior of the current packet.
    // When this signal is encountered, it indicates the presence of
    // additional modifiers or extensions, requiring further processing to
    // adjust or augment the packet's functionality. ModEx can be used to
    // introduce new capabilities or modify existing ones, such as
    // enabling support for high-precision timestamps or other advanced
    // features that enhance the base packet structure.
    ModEx = 7,
    // 8 - Reserved
    // ...
    // 14 - reserved
    // 15 - reserved
}

impl Into<u8> for VideoPacketType {
    fn into(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for VideoPacketType {
    type Error = FLVError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::SequenceStart),
            1 => Ok(Self::CodedFrames),
            2 => Ok(Self::SequenceEnd),
            3 => Ok(Self::CodedFramesX),
            4 => Ok(Self::Metadata),
            5 => Ok(Self::MPEG2TSSequenceStart),
            6 => Ok(Self::Multitrack),
            7 => Ok(Self::ModEx),
            _ => Err(FLVError::UnknownVideoPacketType(value)),
        }
    }
}

impl From<AVCPacketType> for VideoPacketType {
    fn from(value: AVCPacketType) -> Self {
        match value {
            AVCPacketType::SequenceHeader => Self::SequenceStart,
            AVCPacketType::NALU => Self::CodedFrames,
            AVCPacketType::EndOfSequence => Self::SequenceEnd,
        }
    }
}

impl TryInto<AVCPacketType> for VideoPacketType {
    type Error = FLVError;
    fn try_into(self) -> Result<AVCPacketType, Self::Error> {
        match self {
            Self::SequenceStart => Ok(AVCPacketType::SequenceHeader),
            Self::CodedFrames | Self::CodedFramesX => Ok(AVCPacketType::NALU),
            Self::SequenceEnd => Ok(AVCPacketType::EndOfSequence),
            _ => Err(FLVError::UnknownAVCPacketType(255)),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoPacketModExType {
    TimestampOffsetNano = 0,
    // ...
    // 14 - reserved
    // 15 - reserved
}

impl Into<u8> for VideoPacketModExType {
    fn into(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for VideoPacketModExType {
    type Error = FLVError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::TimestampOffsetNano),
            _ => Err(FLVError::UnknownVideoPacketModExType(value)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoFourCC {
    VP8,
    VP9,
    AV1,
    AVC,
    HEVC,
}

pub mod video_four_cc_value {
    use crate::tag::enhanced::make_four_cc;

    pub const VP8_VALUE: u32 = make_four_cc("vp08");
    pub const VP9_VALUE: u32 = make_four_cc("vp09");
    pub const AV1_VALUE: u32 = make_four_cc("av01");
    pub const AVC_VALUE: u32 = make_four_cc("avc1");
    pub const HEVC_VALUE: u32 = make_four_cc("hvc1");
}

impl Into<u32> for VideoFourCC {
    fn into(self) -> u32 {
        match self {
            VideoFourCC::VP8 => video_four_cc_value::VP8_VALUE,
            VideoFourCC::VP9 => video_four_cc_value::VP9_VALUE,
            VideoFourCC::AV1 => video_four_cc_value::AV1_VALUE,
            VideoFourCC::AVC => video_four_cc_value::AVC_VALUE,
            VideoFourCC::HEVC => video_four_cc_value::HEVC_VALUE,
        }
    }
}

impl TryFrom<u32> for VideoFourCC {
    type Error = FLVError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            video_four_cc_value::VP8_VALUE => Ok(Self::VP8),
            video_four_cc_value::VP9_VALUE => Ok(Self::VP9),
            video_four_cc_value::AV1_VALUE => Ok(Self::AV1),
            video_four_cc_value::AVC_VALUE => Ok(Self::AVC),
            video_four_cc_value::HEVC_VALUE => Ok(Self::HEVC),
            _ => Err(FLVError::UnknownFourCC(format!(
                "got unknown fourcc for video codec: {}",
                value
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VideoModEx {
    pub timestamp_nano: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct VideoTrackInfo {
    pub codec: VideoFourCC,
    pub composition_time: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ExVideoTagHeader {
    pub packet_type: VideoPacketType,
    pub frame_type: FrameType,
    pub packet_mod_ex: VideoModEx,
    pub track_type: Option<AvMultiTrackType>,
    pub video_command: Option<VideoCommand>,
    pub tracks: HashMap<u8, VideoTrackInfo>,
}
