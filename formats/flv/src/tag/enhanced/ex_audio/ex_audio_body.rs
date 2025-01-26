use crate::errors::{FLVError, FLVResult};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioChannelOrder {
    // Only the channel count is specified, without any further information about the channel order
    Unspecified = 0,
    // The native channel order (i.e., the channels are in the same order in
    // which as defined in the AudioChannel enum).
    Native = 1,
    // The channel order does not correspond to any predefined
    // order and is stored as an explicit map.
    Custom = 2,
}

impl From<AudioChannelOrder> for u8 {
    fn from(value: AudioChannelOrder) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for AudioChannelOrder {
    type Error = FLVError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Unspecified),
            1 => Ok(Self::Native),
            2 => Ok(Self::Custom),
            _ => Err(FLVError::UnknownChannelOrder(value)),
        }
    }
}

pub mod audio_channel_mask {
    // Mask used to indicate which channels are present in the stream.
    // masks for commonly used speaker configurations
    // <https://en.wikipedia.org/wiki/Surround_sound#Standard_speaker_channels>
    pub const FRONT_LEFT: u32 = 0x000001;
    pub const FRONT_RIGHT: u32 = 0x000002;
    pub const FRONT_CENTER: u32 = 0x000004;
    pub const LOW_FREQUENCY1: u32 = 0x000008;
    pub const BACK_LEFT: u32 = 0x000010;
    pub const BACK_RIGHT: u32 = 0x000020;
    pub const FRONT_LEFT_CENTER: u32 = 0x000040;
    pub const FRONT_RIGHT_CENTER: u32 = 0x000080;
    pub const BACK_CENTER: u32 = 0x000100;
    pub const SIDE_LEFT: u32 = 0x000200;
    pub const SIDE_RIGHT: u32 = 0x000400;
    pub const TOP_CENTER: u32 = 0x000800;
    pub const TOP_FRONT_LEFT: u32 = 0x001000;
    pub const TOP_FRONT_CENTER: u32 = 0x002000;
    pub const TOP_FRONT_RIGHT: u32 = 0x004000;
    pub const TOP_BACK_LEFT: u32 = 0x008000;
    pub const TOP_BACK_CENTER: u32 = 0x010000;
    pub const TOP_BACK_RIGHT: u32 = 0x020000;
    // Completes 22.2 multichannel audio, as
    // standardized in SMPTE ST2036-2-2008
    // see - <https://en.wikipedia.org/wiki/22.2_surround_sound>
    pub const LOW_FREQUENCY2: u32 = 0x040000;
    pub const TOP_SIDE_LEFT: u32 = 0x080000;
    pub const TOP_SIDE_RIGHT: u32 = 0x100000;
    pub const BOTTOM_FRONT_CENTER: u32 = 0x200000;
    pub const BOTTOM_FRONT_LEFT: u32 = 0x400000;
    pub const BOTTOM_FRONT_RIGHT: u32 = 0x800000;
}

/// Channel mappings enums
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioChannel {
    // commonly used speaker configurations
    // see - <https://en.wikipedia.org/wiki/Surround_sound#Standard_speaker_channels>
    FrontLeft = 0, // i.e., FrontLeft is assigned to channel zero
    FrontRight = 1,
    FrontCenter = 2,
    LowFrequency1 = 3,
    BackLeft = 4,
    BackRight = 5,
    FrontLeftCenter = 6,
    FrontRightCenter = 7,
    BackCenter = 8,
    SideLeft = 9,
    SideRight = 10,
    TopCenter = 11,
    TopFrontLeft = 12,
    TopFrontCenter = 13,
    TopFrontRight = 14,
    TopBackLeft = 15,
    TopBackCenter = 16,
    TopBackRight = 17,
    // mappings to complete 22.2 multichannel audio, as
    // standardized in SMPTE ST2036-2-2008
    // see - <https://en.wikipedia.org/wiki/22.2_surround_sound>
    LowFrequency2 = 18,
    TopSideLeft = 19,
    TopSideRight = 20,
    BottomFrontCenter = 21,
    BottomFrontLeft = 22,
    BottomFrontRight = 23,
    // 24 - Reserved
    // ... // 0xfd - reserved
    // Channel is empty and can be safely skipped.
    Unused = 0xfe,
    // Channel contains data, but its speaker configuration is unknown.
    Unknown = 0xff,
}

pub const AUDIO_CHANNEL_INDEXES: [AudioChannel; 24] = [
    AudioChannel::FrontLeft, // = 0, // i.e., FrontLeft is assigned to channel zero
    AudioChannel::FrontRight, // = 1,
    AudioChannel::FrontCenter, //  = 2,
    AudioChannel::LowFrequency1, // = 3,
    AudioChannel::BackLeft,  // = 4,
    AudioChannel::BackRight, // = 5,
    AudioChannel::FrontLeftCenter, // = 6,
    AudioChannel::FrontRightCenter, // = 7,
    AudioChannel::BackCenter, // = 8,
    AudioChannel::SideLeft,  // = 9,
    AudioChannel::SideRight, // = 10,
    AudioChannel::TopCenter, //= 11,
    AudioChannel::TopFrontLeft, // = 12,
    AudioChannel::TopFrontCenter, // = 13,
    AudioChannel::TopFrontRight, // = 14,
    AudioChannel::TopBackLeft, // = 15,
    AudioChannel::TopBackCenter, // = 16,
    AudioChannel::TopBackRight, // = 17,
    AudioChannel::LowFrequency2, // = 18,
    AudioChannel::TopSideLeft, // = 19,
    AudioChannel::TopSideRight, // = 20,
    AudioChannel::BottomFrontCenter, // = 21,
    AudioChannel::BottomFrontLeft, // = 22,
    AudioChannel::BottomFrontRight, // = 23,
];

impl From<AudioChannel> for u8 {
    fn from(value: AudioChannel) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for AudioChannel {
    type Error = FLVError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value < 24 {
            return Ok(AUDIO_CHANNEL_INDEXES[value as usize]);
        }

        if value == AudioChannel::Unused.into() {
            return Ok(AudioChannel::Unused);
        }

        if value == AudioChannel::Unknown.into() {
            return Ok(AudioChannel::Unknown);
        }

        Err(FLVError::UnknownAudioChannel(value))
    }
}

pub fn read_channels_from_mask(mask: u32) -> Vec<AudioChannel> {
    let mut result = Vec::new();
    for (i, audio_channel_idx) in AUDIO_CHANNEL_INDEXES.iter().enumerate() {
        let m = 1 << i;
        if (mask & m) == m {
            result.push(*audio_channel_idx);
        }
    }
    result
}

pub fn make_channel_mask(channel: AudioChannel) -> u32 {
    if channel == AudioChannel::Unused || channel == AudioChannel::Unknown {
        return 0;
    }
    let index: u8 = channel.into();
    1 << index
}

pub fn make_channel_masks(channels: &[AudioChannel]) -> u32 {
    let mut mask = 0;
    for c in channels {
        mask |= make_channel_mask(*c);
    }
    mask
}

#[derive(Debug)]
pub struct AudioMultichannelConfig {
    channel_order: AudioChannelOrder,
    channel_mapping: Vec<AudioChannel>,
}

impl AudioMultichannelConfig {
    pub fn read_from<R>(mut reader: R) -> FLVResult<Self>
    where
        R: io::Read,
    {
        let audio_channel_order: AudioChannelOrder = reader.read_u8()?.try_into()?;
        let channel_cnt = reader.read_u8()? as usize;
        let mapping = match audio_channel_order {
            AudioChannelOrder::Unspecified => vec![AudioChannel::Unknown; channel_cnt],
            AudioChannelOrder::Native => {
                let channel_mask = reader.read_u32::<BigEndian>()?;
                read_channels_from_mask(channel_mask)
            }
            AudioChannelOrder::Custom => {
                let mut channels = vec![0_u8; channel_cnt];
                reader.read_exact(&mut channels)?;
                let mut channel_mapping = Vec::new();
                for v in channels {
                    channel_mapping.push(v.try_into()?);
                }
                channel_mapping
            }
        };
        Ok(Self {
            channel_order: audio_channel_order,
            channel_mapping: mapping,
        })
    }

    pub fn write_to<W>(&self, mut writer: W) -> FLVResult<()>
    where
        W: io::Write,
    {
        writer.write_u8(self.channel_order.into())?;
        writer.write_u8(self.channel_mapping.len() as u8)?;
        match self.channel_order {
            AudioChannelOrder::Unspecified => {}
            AudioChannelOrder::Custom => {
                let buffer: Vec<u8> = self
                    .channel_mapping
                    .iter()
                    .map(|v| <AudioChannel as Into<u8>>::into(*v))
                    .collect();
                writer.write_all(&buffer)?;
            }
            AudioChannelOrder::Native => {
                let mask = make_channel_masks(&self.channel_mapping);
                writer.write_u32::<BigEndian>(mask)?;
            }
        }
        Ok(())
    }
}
