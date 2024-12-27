use std::io;

use crate::errors::{FLVError, FLVResult};

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
pub enum FrameType {
    #[default]
    KeyFrame = 1,
    InterFrame = 2,
    DisposableInterFrame = 3,
    GeneratedKeyFrame = 4,
    CommandFrame = 5,
}

impl Into<u8> for FrameType {
    fn into(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for FrameType {
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
    HEVC = 12, // not standard, but used a lot
}

impl Into<u8> for CodecID {
    fn into(self) -> u8 {
        self as u8
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

impl Into<u8> for AVCPacketType {
    fn into(self) -> u8 {
        self as u8
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

impl Into<u8> for VideoCommand {
    fn into(self) -> u8 {
        self as u8
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
pub struct VideoTagHeader {
    pub frame_type: FrameType,
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

impl VideoTagHeader {
    #[inline]
    pub fn get_frame_type(&self) -> FrameType {
        self.frame_type
    }

    #[inline]
    pub fn get_codec_id(&self) -> CodecID {
        self.codec_id
    }

    #[inline]
    pub fn get_avc_packet_type(&self) -> Option<AVCPacketType> {
        self.avc_packet_type
    }

    #[inline]
    pub fn is_avc(&self) -> bool {
        self.avc_packet_type.is_some()
    }

    #[inline]
    pub fn is_avc_key_frame(&self) -> bool {
        self.frame_type == FrameType::KeyFrame
    }

    #[inline]
    pub fn is_avc_sequence_header(&self) -> bool {
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

    #[inline]
    pub fn is_sequence_header(&self) -> bool {
        if self.is_avc() {
            return self.is_avc_sequence_header();
        }
        // TODO - more codecs
        return false;
    }

    #[inline]
    pub fn is_key_frame(&self) -> bool {
        if self.is_avc() {
            return self.is_avc_key_frame();
        }
        // TODO - more codecs
        return false;
    }
}

impl VideoTagHeader {
    pub fn write_to<W>(&self, writer: W) -> FLVResult<()>
    where
        W: io::Write,
    {
        writer::Writer::new(writer).write(self)
    }
}
