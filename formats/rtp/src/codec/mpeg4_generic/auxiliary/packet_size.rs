use num::ToPrimitive;
use utils::traits::dynamic_sized_packet::DynamicSizedPacket;

use crate::codec::mpeg4_generic::parameters::RtpMpeg4OutOfBandParams;

use super::AuxiliaryData;

pub struct AuxiliaryDataBytesCountWrapper<'a>(
    pub &'a AuxiliaryData,
    pub &'a RtpMpeg4OutOfBandParams,
);

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
