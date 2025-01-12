use utils::traits::dynamic_sized_packet::DynamicSizedPacket;

use crate::header::RtpHeader;

pub(crate) mod padding;
pub trait RtpPacketTrait: DynamicSizedPacket {
    fn get_packet_bytes_count_without_padding(&self) -> usize;
    fn get_header(&self) -> RtpHeader;
}
