use utils::traits::dynamic_sized_packet::DynamicSizedPacket;

use crate::codec::mpeg4_generic::{
    access_unit::packet_size::AccessUnitSectionBytesCountWrapper,
    au_header::packet_size::AuHeaderSectionBytesCountWrapper,
    auxiliary::packet_size::AuxiliaryDataBytesCountWrapper, parameters::RtpMpeg4Fmtp,
};

use super::RtpMpeg4GenericPacket;

pub struct RtpMpeg4GenericPacketBytesCountWrapper<'a>(
    pub &'a RtpMpeg4GenericPacket,
    pub &'a RtpMpeg4Fmtp,
);

impl<'a> DynamicSizedPacket for RtpMpeg4GenericPacketBytesCountWrapper<'a> {
    fn get_packet_bytes_count(&self) -> usize {
        self.0.header.get_packet_bytes_count()
            + self
                .0
                .au_header_section
                .as_ref()
                .map(|h| AuHeaderSectionBytesCountWrapper(h, self.1).get_packet_bytes_count())
                .unwrap_or(0)
            + self
                .0
                .auxiliary_data_section
                .as_ref()
                .map(|a| AuxiliaryDataBytesCountWrapper(a, self.1).get_packet_bytes_count())
                .unwrap_or(0)
            + AccessUnitSectionBytesCountWrapper(&self.0.au_section, self.1)
                .get_packet_bytes_count()
    }
}
