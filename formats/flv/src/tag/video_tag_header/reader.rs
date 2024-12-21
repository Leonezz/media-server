use byteorder::{BigEndian, ReadBytesExt};
use std::io;

use crate::errors::FLVResult;

use super::{CodecID, FrameType, VideoTagHeader};

pub struct Reader<R> {
    inner: R,
}

impl<R> Reader<R>
where
    R: io::Read,
{
    pub fn new(inner: R) -> Self {
        Self { inner }
    }

    pub fn read(&mut self) -> FLVResult<VideoTagHeader> {
        let byte = self.inner.read_u8()?;
        let frame_type: FrameType = ((byte >> 4) & 0b1111).try_into()?;
        let codec_id: CodecID = ((byte >> 0) & 0b1111).try_into()?;
        let mut avc_packet_type = None;
        let mut composition_time = None;
        if codec_id == CodecID::AVC {
            let packet_type = self.inner.read_u8()?;
            avc_packet_type = Some(packet_type.try_into()?);

            let time = self.inner.read_u24::<BigEndian>()?;
            composition_time = Some(time);
        }
        Ok(VideoTagHeader {
            frame_type,
            codec_id,
            avc_packet_type,
            composition_time,
        })
    }
}
