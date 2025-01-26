pub mod reader;
pub mod writer;

use std::io;

use crate::errors::{FLVError, FLVResult};

///
/// Format of SoundData, the following values are defined
/// 0 = Linear PCM, platform endian
/// 1 = ADPCM
/// 2 = MP3
/// 3 = Linear PCM, little endian
/// 4 = Nellymoser 16 kHz mono
/// 5 = Nellymoser 8 kHz mono
/// 6 = Nellymoser
/// 7 = G.711 A-Law logarithmic PCM
/// 8 = G.711 mu-Law logarithmic PCM
/// 9 = reserved
/// 10 = AAC
/// 11 = Speex
/// 14 = MP4 8 kHz
/// 15 = Device-specific sound
/// Formats 7, 8, 14, and 15 are reserved
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub enum SoundFormat {
    LinearPCM = 0,
    ADPCM = 1,
    MP3 = 2,
    LinearPCMLittleEndian = 3,
    NellyMoser16KHZ = 4,
    NellyMoser8KHZ = 5,
    NellyMoser = 6,
    G711ALawLogarithmicPCM = 7,  // reserved
    G711MULawLogarithmicPCM = 8, // reserved
    #[default]
    AAC = 10,
    Speex = 11,
    MP38KHZ = 14, // reserved,
    DeviceSpecific = 15,
}

impl From<SoundFormat> for u8 {
    fn from(value: SoundFormat) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for SoundFormat {
    type Error = FLVError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::LinearPCM),
            1 => Ok(Self::ADPCM),
            2 => Ok(Self::MP3),
            3 => Ok(Self::LinearPCMLittleEndian),
            4 => Ok(Self::NellyMoser16KHZ),
            5 => Ok(Self::NellyMoser8KHZ),
            6 => Ok(Self::NellyMoser),
            7 => Ok(Self::G711ALawLogarithmicPCM),
            8 => Ok(Self::G711MULawLogarithmicPCM),
            10 => Ok(Self::AAC),
            11 => Ok(Self::Speex),
            14 => Ok(Self::MP38KHZ),
            15 => Ok(Self::DeviceSpecific),
            _ => Err(FLVError::UnknownAudioSoundFormat(value)),
        }
    }
}

///
/// Sampling rate. The following values are defined:
/// 0 = 5.5 kHz
/// 1 = 11 kHz
/// 2 = 22 kHz
/// 3 = 44 kHz
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub enum SoundRate {
    KHZ5D5 = 0,
    KHZ11 = 1,
    KHZ22 = 2,
    #[default]
    KHZ44 = 3,
}

impl From<SoundRate> for u8 {
    fn from(value: SoundRate) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for SoundRate {
    type Error = FLVError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::KHZ5D5),
            1 => Ok(Self::KHZ11),
            2 => Ok(Self::KHZ22),
            3 => Ok(Self::KHZ44),
            _ => Err(FLVError::UnknownAudioSoundRate(value)),
        }
    }
}

///
/// Size of each audio sample.
/// This parameter only pertains to uncompressed formats.
/// Compressed formats always decode to 16 bits internally.
/// 0 = 8-bit samples
/// 1 = 16-bit samples
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub enum SoundSize {
    Bit8 = 0,
    #[default]
    Bit16 = 1,
}

impl From<SoundSize> for u8 {
    fn from(value: SoundSize) -> Self {
        value as u8
    }
}

impl From<u8> for SoundSize {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Bit8,
            _ => Self::Bit16,
        }
    }
}

///
/// Mono or stereo sound
/// 0 = Mono sound
/// 1 = Stereo sound
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub enum SoundType {
    Mono = 0,
    #[default]
    Stereo = 1,
}

impl From<SoundType> for u8 {
    fn from(value: SoundType) -> Self {
        value as u8
    }
}

impl From<u8> for SoundType {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Mono,
            _ => Self::Stereo,
        }
    }
}

///
/// IF SoundFormat == 10
/// The following values are defined:
/// 0 = AAC sequence header
/// 1 = AAC raw
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AACPacketType {
    AACSequenceHeader = 0,
    AACRaw = 1,
}

impl From<AACPacketType> for u8 {
    fn from(value: AACPacketType) -> Self {
        value as u8
    }
}

impl From<u8> for AACPacketType {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::AACSequenceHeader,
            _ => Self::AACRaw,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AudioTagHeader {
    pub sound_format: SoundFormat,
    pub sound_rate: SoundRate,
    pub sound_size: SoundSize,
    pub sound_type: SoundType,
    pub aac_packet_type: Option<AACPacketType>,
}

impl AudioTagHeader {
    #[inline]
    pub fn get_sound_format(&self) -> SoundFormat {
        self.sound_format
    }

    #[inline]
    pub fn get_sound_rate(&self) -> SoundRate {
        self.sound_rate
    }

    #[inline]
    pub fn get_sound_size(&self) -> SoundSize {
        self.sound_size
    }

    #[inline]
    pub fn get_sound_type(&self) -> SoundType {
        self.sound_type
    }

    #[inline]
    pub fn get_aac_packet_type(&self) -> Option<AACPacketType> {
        self.aac_packet_type
    }

    #[inline]
    pub fn is_aac(&self) -> bool {
        self.sound_format == SoundFormat::AAC
    }

    #[inline]
    pub fn is_aac_sequence_header(&self) -> bool {
        match self.aac_packet_type {
            None => false,
            Some(packet_type) => packet_type == AACPacketType::AACSequenceHeader,
        }
    }

    #[inline]
    pub fn is_aac_raw(&self) -> bool {
        match self.aac_packet_type {
            None => false,
            Some(packet_type) => packet_type == AACPacketType::AACRaw,
        }
    }

    #[inline]
    pub fn is_sequence_header(&self) -> bool {
        if self.is_aac() {
            return self.is_aac_sequence_header();
        }

        // TODO - more codecs
        false
    }
}

impl AudioTagHeader {
    pub fn write_to<W>(&self, writer: W) -> FLVResult<()>
    where
        W: io::Write,
    {
        writer::Writer::new(writer).write(self)
    }
}
