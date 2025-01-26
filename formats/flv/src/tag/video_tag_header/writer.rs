use std::io;

use byteorder::{BigEndian, WriteBytesExt};

use crate::errors::{FLVError, FLVResult};

use super::{CodecID, FrameType, VideoTagHeader};

#[derive(Debug)]
pub struct Writer<W> {
    inner: W,
}

impl<W> Writer<W>
where
    W: io::Write,
{
    pub fn new(inner: W) -> Self {
        Self { inner }
    }

    pub fn write(&mut self, header: &VideoTagHeader) -> FLVResult<()> {
        if (header.codec_id == CodecID::AVC
            || header.codec_id == CodecID::HEVC
            || header.codec_id == CodecID::AV1)
            && (header.avc_packet_type.is_none() || header.composition_time.is_none())
        {
            return Err(FLVError::InconsistentHeader(
                "video header with codec id 7 (AVC) should also has avc packet type and composition_time".to_owned()
            ));
        }

        if header.frame_type == FrameType::CommandFrame && header.video_command.is_none() {
            return Err(FLVError::InconsistentHeader(
                "video header with frame type: 5 (VideoCommand) should also has video_command"
                    .to_owned(),
            ));
        }

        let mut byte = 0;
        byte |= <FrameType as Into<u8>>::into(header.frame_type);
        byte <<= 4;
        byte |= <CodecID as Into<u8>>::into(header.codec_id);

        self.inner.write_u8(byte)?;

        if header.frame_type == FrameType::CommandFrame {
            let command: u8 = header
                .video_command
                .expect("video command cannot be none")
                .into();
            self.inner.write_u8(command)?;
        }

        if header.codec_id == CodecID::AVC
            || header.codec_id == CodecID::HEVC
            || header.codec_id == CodecID::AV1
        {
            let avc_packet_type_u8: u8 =
                header.avc_packet_type.expect("this cannot be none").into();
            self.inner.write_u8(avc_packet_type_u8)?;
            let composition_time_u32 = header.composition_time.expect("this cannot be none");
            self.inner.write_u24::<BigEndian>(composition_time_u32)?;
        }

        Ok(())
    }
}
