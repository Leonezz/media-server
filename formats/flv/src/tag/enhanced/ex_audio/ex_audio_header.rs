use std::collections::HashMap;

use crate::{
    errors::FLVError,
    tag::{
        audio_tag_header::{self, AACPacketType},
        enhanced::AvMultiTrackType,
    },
};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AudioPacketType {
    #[default]
    SequenceStart = 0,
    CodedFrames = 1,
    SequenceEnd = 2,
    // 3 - reserved,
    MultichannelConfig = 4,
    MultiTrack = 5,
    // 6 - reserved

    // ModEx is a special signal within the AudioPacketType enum that
    // serves to both modify and extend the behavior of the current packet.
    // When this signal is encountered, it indicates the presence of
    // additional modifiers or extensions, requiring further processing to
    // adjust or augment the packet's functionality. ModEx can be used to
    // introduce new capabilities or modify existing ones, such as
    // enabling support for high-precision timestamps or other advanced
    // features that enhance the base packet structure.
    ModEx = 7,
}

impl Into<u8> for AudioPacketType {
    fn into(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for AudioPacketType {
    type Error = FLVError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::SequenceStart),
            1 => Ok(Self::CodedFrames),
            2 => Ok(Self::SequenceEnd),
            4 => Ok(Self::MultichannelConfig),
            5 => Ok(Self::MultiTrack),
            7 => Ok(Self::ModEx),
            _ => Err(FLVError::UnknownAVCPacketType(value)),
        }
    }
}

impl From<audio_tag_header::AACPacketType> for AudioPacketType {
    fn from(value: audio_tag_header::AACPacketType) -> Self {
        match value {
            AACPacketType::AACSequenceHeader => Self::SequenceStart,
            AACPacketType::AACRaw => Self::CodedFrames,
        }
    }
}

impl TryInto<audio_tag_header::AACPacketType> for AudioPacketType {
    type Error = FLVError;
    fn try_into(self) -> Result<audio_tag_header::AACPacketType, Self::Error> {
        match self {
            Self::SequenceStart => Ok(audio_tag_header::AACPacketType::AACSequenceHeader),
            Self::CodedFrames => Ok(audio_tag_header::AACPacketType::AACRaw),
            _ => Err(FLVError::UnknownAudioPacketType(255)),
        }
    }
}

#[repr(u8)]
#[derive(Debug)]
pub enum AudioPacketModExType {
    TimestampOffsetNano = 0,
}

impl Into<u8> for AudioPacketModExType {
    fn into(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for AudioPacketModExType {
    type Error = FLVError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::TimestampOffsetNano),
            _ => Err(FLVError::UnknownAudioPacketModExType(value)),
        }
    }
}

/// Valid FOURCC values for signaling support of audio codecs
/// in the enhanced FourCC pipeline. In this context, support
/// for a FourCC codec MUST be signaled via the enhanced
/// "connect" command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFourCC {
    // AC-3/E-AC-3 - <https://en.wikipedia.org/wiki/Dolby_Digital>
    AC3,
    EAC3,
    // Opus audio - <https://opus-codec.org/>
    OPUS,
    // Mp3 audio - <https://en.wikipedia.org/wiki/MP3>
    MP3,
    // Free Lossless Audio Codec - <https://xiph.org/flac/format.html>
    FLAC,
    // Advanced Audio Coding - <https://en.wikipedia.org/wiki/Advanced_Audio_Coding>
    // The following AAC profiles, denoted by their object types, are supported
    // 1 = main profile
    // 2 = low complexity, a.k.a., LC
    // 5 = high efficiency / scale band replication, a.k.a., HE / SBR
    AAC,
}

pub mod audio_four_cc_value {
    use crate::tag::enhanced::make_four_cc;

    pub const AC3_VALUE: u32 = make_four_cc("ac-3");
    pub const EAC3_VALUE: u32 = make_four_cc("ec-3");
    pub const OPUS_VALUE: u32 = make_four_cc("Opus");
    pub const MP3_VALUE: u32 = make_four_cc(".mp3");
    pub const FLAC_VALUE: u32 = make_four_cc("fLaC");
    pub const AAC_VALUE: u32 = make_four_cc("mp4a");
}

impl Into<u32> for AudioFourCC {
    fn into(self) -> u32 {
        match self {
            Self::AC3 => audio_four_cc_value::AC3_VALUE,
            Self::EAC3 => audio_four_cc_value::EAC3_VALUE,
            Self::OPUS => audio_four_cc_value::OPUS_VALUE,
            Self::MP3 => audio_four_cc_value::MP3_VALUE,
            Self::FLAC => audio_four_cc_value::FLAC_VALUE,
            Self::AAC => audio_four_cc_value::AAC_VALUE,
        }
    }
}

impl TryFrom<u32> for AudioFourCC {
    type Error = FLVError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            audio_four_cc_value::AC3_VALUE => Ok(Self::AC3),
            audio_four_cc_value::EAC3_VALUE => Ok(Self::EAC3),
            audio_four_cc_value::OPUS_VALUE => Ok(Self::OPUS),
            audio_four_cc_value::MP3_VALUE => Ok(Self::MP3),
            audio_four_cc_value::FLAC_VALUE => Ok(Self::FLAC),
            audio_four_cc_value::AAC_VALUE => Ok(Self::AAC),
            _ => Err(FLVError::UnknownFourCC(format!(
                "got unknown fourcc for audio codec: {}",
                value
            ))),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AudioModEx {
    pub timestamp_nano: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct AudioTrackInfo {
    pub codec: AudioFourCC,
    // payload: BytesMut,
}

#[derive(Debug, Clone, Default)]
pub struct ExAudioTagHeader {
    pub packet_type: AudioPacketType,
    pub packet_mod_ex: AudioModEx,
    pub track_type: Option<AvMultiTrackType>,

    pub tracks: HashMap<u8, AudioTrackInfo>,
}
