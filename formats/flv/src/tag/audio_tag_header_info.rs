use std::collections::HashMap;

use tokio_util::either::Either;

use crate::errors::FLVError;

use super::{
    audio_tag_header::{self, SoundRate, SoundSize, SoundType},
    enhanced::{
        AvMultiTrackType,
        ex_audio::ex_audio_header::{
            AudioFourCC, AudioModEx, AudioPacketType, AudioTrackInfo, ExAudioTagHeader,
        },
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioCodecCommon {
    LinearPCM,
    ADPCM,
    MP3,
    LinearPCMLittleEndian,
    NellyMoser16KHZ,
    NellyMoser8KHZ,
    NellyMoser,
    G711ALawLogarithmicPCM,  // reserved
    G711MULawLogarithmicPCM, // reserved
    AAC,
    Speex,
    MP38KHZ, // reserved,
    DeviceSpecific,
    AC3,
    EAC3,
    OPUS,
    FLAC,
}

impl TryInto<audio_tag_header::SoundFormat> for AudioCodecCommon {
    type Error = FLVError;
    fn try_into(self) -> Result<audio_tag_header::SoundFormat, Self::Error> {
        match self {
            Self::LinearPCM => Ok(audio_tag_header::SoundFormat::LinearPCM),
            Self::ADPCM => Ok(audio_tag_header::SoundFormat::ADPCM),
            Self::MP3 => Ok(audio_tag_header::SoundFormat::MP3),
            Self::LinearPCMLittleEndian => Ok(audio_tag_header::SoundFormat::LinearPCMLittleEndian),
            Self::NellyMoser16KHZ => Ok(audio_tag_header::SoundFormat::NellyMoser16KHZ),
            Self::NellyMoser8KHZ => Ok(audio_tag_header::SoundFormat::NellyMoser8KHZ),
            Self::NellyMoser => Ok(audio_tag_header::SoundFormat::NellyMoser),
            Self::G711ALawLogarithmicPCM => {
                Ok(audio_tag_header::SoundFormat::G711ALawLogarithmicPCM)
            }
            Self::G711MULawLogarithmicPCM => {
                Ok(audio_tag_header::SoundFormat::G711MULawLogarithmicPCM)
            }
            Self::AAC => Ok(audio_tag_header::SoundFormat::AAC),
            Self::Speex => Ok(audio_tag_header::SoundFormat::Speex),
            Self::MP38KHZ => Ok(audio_tag_header::SoundFormat::MP38KHZ),
            Self::DeviceSpecific => Ok(audio_tag_header::SoundFormat::DeviceSpecific),
            _ => Err(FLVError::UnknownAudioSoundFormat(255)),
        }
    }
}

impl TryInto<AudioFourCC> for AudioCodecCommon {
    type Error = FLVError;
    fn try_into(self) -> Result<AudioFourCC, Self::Error> {
        match self {
            Self::AAC => Ok(AudioFourCC::AAC),
            Self::EAC3 => Ok(AudioFourCC::EAC3),
            Self::OPUS => Ok(AudioFourCC::OPUS),
            Self::MP3 => Ok(AudioFourCC::MP3),
            Self::AC3 => Ok(AudioFourCC::AC3),
            Self::FLAC => Ok(AudioFourCC::FLAC),

            _ => Err(FLVError::UnknownFourCC(format!(
                "trying to convert legacy sound format: {:?} to AudioFourCC",
                self
            ))),
        }
    }
}

impl From<audio_tag_header::SoundFormat> for AudioCodecCommon {
    fn from(value: audio_tag_header::SoundFormat) -> AudioCodecCommon {
        match value {
            audio_tag_header::SoundFormat::AAC => Self::AAC,
            audio_tag_header::SoundFormat::ADPCM => Self::ADPCM,
            audio_tag_header::SoundFormat::DeviceSpecific => Self::DeviceSpecific,
            audio_tag_header::SoundFormat::G711ALawLogarithmicPCM => Self::G711ALawLogarithmicPCM,
            audio_tag_header::SoundFormat::G711MULawLogarithmicPCM => Self::G711MULawLogarithmicPCM,
            audio_tag_header::SoundFormat::LinearPCM => Self::LinearPCM,
            audio_tag_header::SoundFormat::LinearPCMLittleEndian => Self::LinearPCMLittleEndian,
            audio_tag_header::SoundFormat::MP3 => Self::MP3,
            audio_tag_header::SoundFormat::MP38KHZ => Self::MP38KHZ,
            audio_tag_header::SoundFormat::NellyMoser => Self::NellyMoser,
            audio_tag_header::SoundFormat::NellyMoser16KHZ => Self::NellyMoser16KHZ,
            audio_tag_header::SoundFormat::NellyMoser8KHZ => Self::NellyMoser8KHZ,
            audio_tag_header::SoundFormat::Speex => Self::Speex,
        }
    }
}

impl From<AudioFourCC> for AudioCodecCommon {
    fn from(value: AudioFourCC) -> Self {
        match value {
            AudioFourCC::AAC => Self::AAC,
            AudioFourCC::AC3 => Self::AC3,
            AudioFourCC::EAC3 => Self::EAC3,
            AudioFourCC::FLAC => Self::FLAC,
            AudioFourCC::MP3 => Self::MP3,
            AudioFourCC::OPUS => Self::OPUS,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LegacyAudioHeaderInfo {
    pub sound_rate: SoundRate,
    pub sound_size: SoundSize,
    pub sound_type: SoundType,
}

#[derive(Debug, Clone, Copy)]
pub struct AudioTagHeaderWithoutMultiTrack {
    pub packet_type: AudioPacketType,
    pub codec_id: AudioCodecCommon,
    pub legacy_info: Option<LegacyAudioHeaderInfo>,
    pub timestamp_nano: Option<u32>,
    pub track_type: Option<AvMultiTrackType>,
}

impl TryInto<audio_tag_header::AudioTagHeader> for AudioTagHeaderWithoutMultiTrack {
    type Error = FLVError;
    fn try_into(self) -> Result<audio_tag_header::AudioTagHeader, Self::Error> {
        let legacy_info = self.legacy_info;
        if legacy_info.is_none() {
            return Err(FLVError::InconsistentHeader(format!(
                "trying to convert to legacy audio tag header while there are no legacy info"
            )));
        }
        let legacy_info = legacy_info.unwrap();
        let sound_format: audio_tag_header::SoundFormat = self.codec_id.try_into()?;
        let aac_packet_type: audio_tag_header::AACPacketType = self.packet_type.try_into()?;
        Ok(audio_tag_header::AudioTagHeader {
            sound_format,
            sound_rate: legacy_info.sound_rate,
            sound_size: legacy_info.sound_size,
            sound_type: legacy_info.sound_type,
            aac_packet_type: Some(aac_packet_type),
        })
    }
}

impl TryInto<ExAudioTagHeader> for AudioTagHeaderWithoutMultiTrack {
    type Error = FLVError;
    fn try_into(self) -> Result<ExAudioTagHeader, Self::Error> {
        let mut tracks: HashMap<u8, AudioTrackInfo> = HashMap::new();
        tracks.insert(0, AudioTrackInfo {
            codec: self.codec_id.try_into()?,
        });

        Ok(ExAudioTagHeader {
            packet_type: self.packet_type,
            packet_mod_ex: AudioModEx {
                timestamp_nano: self.timestamp_nano,
            },
            track_type: self.track_type,
            tracks,
        })
    }
}

impl From<audio_tag_header::AudioTagHeader> for AudioTagHeaderWithoutMultiTrack {
    fn from(value: audio_tag_header::AudioTagHeader) -> Self {
        let packet_type = if let Some(packet_type) = value.aac_packet_type {
            packet_type.into()
        } else {
            AudioPacketType::CodedFrames
        };
        Self {
            packet_type,
            codec_id: value.sound_format.into(),
            legacy_info: Some(LegacyAudioHeaderInfo {
                sound_rate: value.sound_rate,
                sound_size: value.sound_size,
                sound_type: value.sound_type,
            }),
            timestamp_nano: None,
            track_type: None,
        }
    }
}

impl TryFrom<ExAudioTagHeader> for AudioTagHeaderWithoutMultiTrack {
    type Error = FLVError;
    fn try_from(value: ExAudioTagHeader) -> Result<Self, Self::Error> {
        let track_info = value.tracks.get(&0);
        if track_info.is_none() {
            return Err(FLVError::InconsistentHeader(format!(
                "expect a valid ExAudioHeader, got {:?} instead",
                value
            )));
        }
        let track_info = track_info.unwrap();

        Ok(Self {
            packet_type: value.packet_type,
            codec_id: track_info.codec.into(),
            legacy_info: None,
            timestamp_nano: value.packet_mod_ex.timestamp_nano,
            track_type: value.track_type,
        })
    }
}

impl AudioTagHeaderWithoutMultiTrack {
    pub fn is_sequence_header(&self) -> bool {
        match self.packet_type {
            AudioPacketType::SequenceStart => true,
            _ => false,
        }
    }

    pub fn get_codec_id(&self) -> AudioCodecCommon {
        self.codec_id
    }
}

impl TryFrom<Either<audio_tag_header::AudioTagHeader, ExAudioTagHeader>>
    for AudioTagHeaderWithoutMultiTrack
{
    type Error = FLVError;
    fn try_from(
        value: Either<audio_tag_header::AudioTagHeader, ExAudioTagHeader>,
    ) -> Result<Self, Self::Error> {
        match value {
            Either::Left(header) => Ok(header.into()),
            Either::Right(header) => Ok(header.try_into()?),
        }
    }
}
