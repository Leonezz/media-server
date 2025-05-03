use crate::{
    errors::FLVError,
    tag::enhanced::{
        AvMultiTrackType,
        ex_audio::ex_audio_header::{
            AudioFourCC, AudioPacketModExType, AudioPacketType, AudioTrackInfo,
        },
    },
};
use byteorder::{BigEndian, ReadBytesExt};
use num::ToPrimitive;
use std::{
    collections::HashMap,
    io::{self, Cursor},
};
use utils::traits::reader::{ReadFrom, ReadRemainingFrom};

use super::{
    ex_audio_body::{
        AudioChannel, AudioChannelOrder, AudioMultichannelConfig, read_channels_from_mask,
    },
    ex_audio_header::{AudioModEx, ExAudioTagHeader},
};

impl<R: io::Read> ReadRemainingFrom<u8, R> for ExAudioTagHeader {
    type Error = FLVError;
    fn read_remaining_from(header: u8, mut reader: R) -> Result<Self, Self::Error> {
        let mut audio_packet_type: AudioPacketType = (header & 0b1111).try_into()?;
        let mut timestamp_nano: Option<u32> = None;
        let mut audio_four_cc: AudioFourCC = AudioFourCC::AAC; // this default value would never be used
        let mut audio_multi_track_type: Option<AvMultiTrackType> = None;

        while audio_packet_type == AudioPacketType::ModEx {
            let mut mod_ex_data_size = reader.read_u8()? as u32 + 1;
            if mod_ex_data_size == 256 {
                mod_ex_data_size = reader.read_u16::<BigEndian>()? as u32 + 1;
            }

            let mut mod_ex_data = vec![0_u8; mod_ex_data_size as usize];
            reader.read_exact(&mut mod_ex_data)?;
            let mut mod_ex_cursor = Cursor::new(&mod_ex_data);

            let byte = reader.read_u8()?;
            let audio_packet_mod_ex_type: AudioPacketModExType =
                ((byte >> 4) & 0b1111).try_into()?;

            audio_packet_type = (byte & 0b1111).try_into()?;

            match audio_packet_mod_ex_type {
                AudioPacketModExType::TimestampOffsetNano => {
                    timestamp_nano = Some(mod_ex_cursor.read_u24::<BigEndian>()?);
                } // will there be other extensions in the future?
            }
        }

        if audio_packet_type == AudioPacketType::MultiTrack {
            let byte = reader.read_u8()?;
            audio_multi_track_type = Some(((byte >> 4) & 0b1111).try_into()?);

            audio_packet_type = (byte & 0b1111).try_into()?;
            if audio_packet_type == AudioPacketType::MultiTrack {
                return Err(FLVError::UnknownAudioPacketType(audio_packet_type.into()));
            }

            if audio_multi_track_type.unwrap() != AvMultiTrackType::ManyTracksManyCodecs {
                audio_four_cc = reader.read_u32::<BigEndian>()?.try_into()?;
            }
        } else {
            audio_four_cc = reader.read_u32::<BigEndian>()?.try_into()?;
        }

        let mut tracks: HashMap<u8, AudioTrackInfo> = HashMap::new();

        loop {
            match audio_multi_track_type {
                None => {
                    tracks.insert(
                        0,
                        AudioTrackInfo {
                            codec: audio_four_cc,
                        },
                    );
                    break;
                }
                Some(multi_track_type) => {
                    if multi_track_type == AvMultiTrackType::ManyTracksManyCodecs {
                        audio_four_cc = reader.read_u32::<BigEndian>()?.try_into()?;
                    }

                    let track_id = reader.read_u8()?;
                    tracks.insert(
                        track_id,
                        AudioTrackInfo {
                            codec: audio_four_cc,
                        },
                    );
                    if multi_track_type != AvMultiTrackType::OneTrack {
                        let track_data_size = reader.read_u24::<BigEndian>()?;
                        let mut buf = vec![0; track_data_size.to_usize().unwrap()];
                        reader.read_exact(&mut buf)?;
                    } else {
                        break;
                    }
                }
            }
        }
        Ok(Self {
            packet_type: audio_packet_type,
            packet_mod_ex: AudioModEx { timestamp_nano },
            track_type: audio_multi_track_type,
            tracks,
        })
    }
}

impl<R: io::Read> ReadFrom<R> for AudioMultichannelConfig {
    type Error = FLVError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
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
}
