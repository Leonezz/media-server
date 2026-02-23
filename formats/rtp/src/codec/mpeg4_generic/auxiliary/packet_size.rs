use super::AuxiliaryData;
use crate::codec::mpeg4_generic::parameters::rfc3640::RtpMpeg4Fmtp;
use num::ToPrimitive;
use utils::traits::dynamic_sized_packet::DynamicSizedPacket;

pub struct AuxiliaryDataBytesCountWrapper<'a>(pub &'a AuxiliaryData, pub &'a RtpMpeg4Fmtp);

impl<'a> DynamicSizedPacket for AuxiliaryDataBytesCountWrapper<'a> {
    fn get_packet_bytes_count(&self) -> usize {
        if let Some(length) = self.1.auxiliary_data_size_length {
            return (self.0.auxiliary_data_size.div_ceil(8) + length)
                .to_usize()
                .expect("integer overflow usize");
        }
        0
    }
}
