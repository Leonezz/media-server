use byteorder::{BigEndian, ReadBytesExt};
use std::io::{self};
use utils::traits::reader::{ReadFrom, ReadRemainingFrom};

use crate::{errors::FLVError, tag::enhanced::ex_video::ex_video_header::ExVideoTagHeader};

use super::{CodecID, FrameTypeFLV, LegacyVideoTagHeader, VideoTagHeader};

impl<R: io::Read> ReadFrom<R> for VideoTagHeader {
    type Error = FLVError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let byte = reader.read_u8()?;

        let is_ex_header = ((byte >> 7) & 0b1) == 0b1;
        if is_ex_header {
            let ex_header = ExVideoTagHeader::read_remaining_from(byte, reader)?;
            return Ok(VideoTagHeader::Enhanced(ex_header));
        }

        let frame_type: FrameTypeFLV = ((byte >> 4) & 0b1111).try_into()?;
        let codec_id: CodecID = (byte & 0b1111).try_into()?;

        let mut video_command = None;
        if frame_type == FrameTypeFLV::CommandFrame {
            video_command = Some(reader.read_u8()?.try_into()?);
        }

        let mut avc_packet_type = None;
        let mut composition_time = None;
        if codec_id == CodecID::AVC || codec_id == CodecID::HEVC || codec_id == CodecID::AV1 {
            let packet_type = reader.read_u8()?;
            avc_packet_type = Some(packet_type.try_into()?);

            let time = reader.read_u24::<BigEndian>()?;
            composition_time = Some(time);
        }
        Ok(VideoTagHeader::Legacy(LegacyVideoTagHeader {
            frame_type,
            codec_id,
            video_command,
            avc_packet_type,
            composition_time,
        }))
    }
}
