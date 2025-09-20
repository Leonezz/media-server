use flv_tag_body::FLVTagBodyWithFilter;
use flv_tag_header::FLVTagHeader;
use utils::traits::{dynamic_sized_packet::DynamicSizedPacket, fixed_packet::FixedPacket};

pub mod audio_tag_header;
pub mod audio_tag_header_info;
pub mod encryption;
pub mod enhanced;
pub mod flv_tag_body;
pub mod flv_tag_header;
pub mod framed;
pub mod on_meta_data;
pub mod reader;
pub mod video_tag_header;
pub mod video_tag_header_info;
pub mod writer;

#[derive(Debug)]
pub struct FLVTag {
    pub tag_header: FLVTagHeader,
    pub body_with_filter: FLVTagBodyWithFilter,
}

impl DynamicSizedPacket for FLVTag {
    fn get_packet_bytes_count(&self) -> usize {
        FLVTagHeader::bytes_count() + self.tag_header.data_size as usize
    }
}
