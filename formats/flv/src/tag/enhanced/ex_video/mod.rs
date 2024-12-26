use ex_video_header::{ExVideoTagHeader, VideoPacketType};

use crate::tag::video_tag_header::FrameType;

pub mod ex_video_header;
pub mod reader;
pub mod writer;

impl ExVideoTagHeader {
    #[inline]
    pub fn is_sequence_header(&self) -> bool {
        match self.packet_type {
            VideoPacketType::SequenceStart => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_key_frame(&self) -> bool {
        match self.frame_type {
            FrameType::KeyFrame => true,
            _ => false,
        }
    }
}
