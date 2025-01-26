use byteorder::{BigEndian, ReadBytesExt};
use std::io::Cursor;
use tokio_util::{bytes::BytesMut, either::Either};

use crate::{errors::FLVResult, tag::enhanced::ex_video::ex_video_header::ExVideoTagHeader};

use super::{CodecID, FrameType, VideoTagHeader};

impl VideoTagHeader {
    pub fn read_from(
        reader: &mut Cursor<&mut BytesMut>,
    ) -> FLVResult<Either<VideoTagHeader, ExVideoTagHeader>> {
        let byte = reader.read_u8()?;

        let is_ex_header = ((byte >> 7) & 0b1) == 0b1;
        if is_ex_header {
            let ex_header = ExVideoTagHeader::read_from(reader, byte)?;
            return Ok(Either::Right(ex_header));
        }

        let frame_type: FrameType = ((byte >> 4) & 0b1111).try_into()?;
        let codec_id: CodecID = (byte & 0b1111).try_into()?;

        let mut video_command = None;
        if frame_type == FrameType::CommandFrame {
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
        Ok(Either::Left(VideoTagHeader {
            frame_type,
            codec_id,
            video_command,
            avc_packet_type,
            composition_time,
        }))
    }
}
