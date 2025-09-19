use ex_video_header::{ExVideoTagHeader, VideoPacketType};

use crate::tag::video_tag_header::FrameTypeFLV;

pub mod ex_video_header;
pub mod reader;
pub mod writer;

impl ExVideoTagHeader {
    #[inline]
    pub fn is_sequence_header(&self) -> bool {
        matches!(self.packet_type, VideoPacketType::SequenceStart)
    }

    #[inline]
    pub fn is_key_frame(&self) -> bool {
        matches!(self.frame_type, FrameTypeFLV::KeyFrame)
    }
}
