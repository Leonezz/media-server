use std::{
    collections::HashMap,
    io::{self, Cursor},
};

use byteorder::{BigEndian, ReadBytesExt};
use num::ToPrimitive;
use utils::traits::reader::ReadRemainingFrom;

use crate::{
    errors::FLVError,
    tag::{
        enhanced::{
            AvMultiTrackType,
            ex_video::ex_video_header::{
                VideoFourCC, VideoModEx, VideoPacketModExType, VideoPacketType, VideoTrackInfo,
            },
        },
        video_tag_header::{FrameTypeFLV, VideoCommand},
    },
};

use super::ex_video_header::ExVideoTagHeader;

impl<R: io::Read> ReadRemainingFrom<u8, R> for ExVideoTagHeader {
    type Error = FLVError;
    fn read_remaining_from(header: u8, reader: &mut R) -> Result<Self, Self::Error> {
        assert!(((header >> 7) & 0b1) == 0b1);
        let video_frame_type: FrameTypeFLV = ((header >> 4) & 0b111).try_into()?;
        let mut video_packet_type: VideoPacketType = (header & 0b1111).try_into()?;
        let mut timestamp_nano = None;
        while video_packet_type == VideoPacketType::ModEx {
            let mut mod_ex_data_size: u32 = reader.read_u8()? as u32 + 1;
            if mod_ex_data_size == 256 {
                mod_ex_data_size = reader.read_u16::<BigEndian>()? as u32 + 1;
            }
            let mut mod_ex_data = vec![0_u8; mod_ex_data_size as usize];
            reader.read_exact(&mut mod_ex_data)?;
            let mut cursor = Cursor::new(&mut mod_ex_data);

            let byte = reader.read_u8()?;
            let video_packet_mod_ex_type: VideoPacketModExType =
                ((byte >> 4) & 0b1111).try_into()?;
            video_packet_type = (byte & 0b11111).try_into()?;

            if video_packet_mod_ex_type == VideoPacketModExType::TimestampOffsetNano {
                timestamp_nano = Some(cursor.read_u24::<BigEndian>()?);
            }
        }

        let mut video_command: Option<VideoCommand> = None;
        let mut video_multi_track_type: Option<AvMultiTrackType> = None;
        let mut video_four_cc: VideoFourCC = VideoFourCC::HEVC; // this default value will never be used

        if video_packet_type != VideoPacketType::Metadata
            && video_frame_type == FrameTypeFLV::CommandFrame
        {
            video_command = Some(reader.read_u8()?.try_into()?);
        } else if video_packet_type == VideoPacketType::Multitrack {
            let byte = reader.read_u8()?;

            video_multi_track_type = Some(((byte >> 4) & 0b1111).try_into()?);
            video_packet_type = (byte & 0b1111).try_into()?;

            assert!(video_packet_type != VideoPacketType::Multitrack);

            if video_multi_track_type.unwrap() != AvMultiTrackType::ManyTracksManyCodecs {
                video_four_cc = reader.read_u32::<BigEndian>()?.try_into()?;
            }
        } else {
            video_four_cc = reader.read_u32::<BigEndian>()?.try_into()?;
        }

        let mut tracks: HashMap<u8, VideoTrackInfo> = HashMap::new();

        loop {
            let (track_id, track_data_size) = match video_multi_track_type {
                None => (0_u8, None),
                Some(multi_track_type) => {
                    if multi_track_type == AvMultiTrackType::ManyTracksManyCodecs {
                        video_four_cc = reader.read_u32::<BigEndian>()?.try_into()?;
                    }
                    let id = reader.read_u8()?;
                    let mut size = None;
                    if multi_track_type != AvMultiTrackType::OneTrack {
                        size = Some(reader.read_u24::<BigEndian>()?);
                    }
                    (id, size)
                }
            };

            let mut composition_time = None;
            if video_packet_type == VideoPacketType::CodedFrames
                && video_four_cc == VideoFourCC::AVC
                || video_four_cc == VideoFourCC::HEVC
            {
                composition_time = Some(reader.read_u24::<BigEndian>()?);
            }

            tracks.insert(
                track_id,
                VideoTrackInfo {
                    codec: video_four_cc,
                    composition_time,
                },
            );

            match track_data_size {
                None => {
                    break;
                }
                Some(size) => {
                    let mut buf = vec![0; size.to_usize().unwrap()];
                    reader.read_exact(&mut buf)?;
                }
            }
        }
        Ok(ExVideoTagHeader {
            packet_type: video_packet_type,
            frame_type: video_frame_type,
            packet_mod_ex: VideoModEx { timestamp_nano },
            track_type: video_multi_track_type,
            video_command,
            tracks,
        })
    }
}
