use std::io;

use crate::errors::FLVError;
use byteorder::{BigEndian, WriteBytesExt};
use utils::traits::writer::WriteTo;

use super::{CodecID, FrameTypeFLV, LegacyVideoTagHeader};

impl<W: io::Write> WriteTo<W> for LegacyVideoTagHeader {
    type Error = FLVError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        if (self.codec_id == CodecID::AVC
            || self.codec_id == CodecID::HEVC
            || self.codec_id == CodecID::AV1)
            && (self.avc_packet_type.is_none() || self.composition_time.is_none())
        {
            return Err(FLVError::InconsistentHeader(
                "video header with codec id 7 (AVC) should also has avc packet type and composition_time".to_owned()
            ));
        }

        if self.frame_type == FrameTypeFLV::CommandFrame && self.video_command.is_none() {
            return Err(FLVError::InconsistentHeader(
                "video header with frame type: 5 (VideoCommand) should also has video_command"
                    .to_owned(),
            ));
        }

        let mut byte = 0;
        byte |= <FrameTypeFLV as Into<u8>>::into(self.frame_type);
        byte <<= 4;
        byte |= <CodecID as Into<u8>>::into(self.codec_id);

        writer.write_u8(byte)?;

        if self.frame_type == FrameTypeFLV::CommandFrame {
            let command: u8 = self
                .video_command
                .expect("video command cannot be none")
                .into();
            writer.write_u8(command)?;
        }

        if self.codec_id == CodecID::AVC
            || self.codec_id == CodecID::HEVC
            || self.codec_id == CodecID::AV1
        {
            let avc_packet_type_u8: u8 = self.avc_packet_type.expect("this cannot be none").into();
            writer.write_u8(avc_packet_type_u8)?;
            let composition_time_u32 = self.composition_time.expect("this cannot be none");
            writer.write_u24::<BigEndian>(composition_time_u32)?;
        }

        Ok(())
    }
}
