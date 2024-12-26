use std::{
    collections::HashMap,
    io::{Cursor, Read, Seek, SeekFrom},
};

use byteorder::{BigEndian, ReadBytesExt};
use tokio_util::bytes::{Buf, BytesMut};

use crate::{
    errors::FLVResult,
    tag::{
        enhanced::{
            AvMultiTrackType,
            ex_video::ex_video_header::{
                VideoFourCC, VideoModEx, VideoPacketModExType, VideoPacketType, VideoTrackInfo,
            },
        },
        video_tag_header::{FrameType, VideoCommand},
    },
};

use super::ex_video_header::ExVideoTagHeader;

impl ExVideoTagHeader {
    pub fn read_from(
        reader: &mut Cursor<&mut BytesMut>,
        first_byte: u8,
    ) -> FLVResult<ExVideoTagHeader> {
        assert!(((first_byte >> 7) & 0b1) == 0b1);
        let video_frame_type: FrameType = ((first_byte >> 4) & 0b111).try_into()?;
        let mut video_packet_type: VideoPacketType = (first_byte & 0b1111).try_into()?;
        let mut timestamp_nano = None;
        while video_packet_type == VideoPacketType::ModEx {
            let mut mod_ex_data_size: u32 = reader.read_u8()? as u32 + 1;
            if mod_ex_data_size == 256 {
                mod_ex_data_size = reader.read_u16::<BigEndian>()? as u32 + 1;
            }
            let mut mod_ex_data = vec![0 as u8; mod_ex_data_size as usize];
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
            && video_frame_type == FrameType::CommandFrame
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
                None => (0 as u8, None),
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

            tracks.insert(track_id, VideoTrackInfo {
                codec: video_four_cc,
                composition_time: composition_time,
            });

            match track_data_size {
                None => {
                    break;
                }
                Some(size) => {
                    if reader.remaining() < size as usize {
                        break;
                    }
                    reader.seek(SeekFrom::Current(size as i64))?;
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
