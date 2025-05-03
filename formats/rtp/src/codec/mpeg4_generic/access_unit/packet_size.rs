use tokio_util::either::Either;
use utils::traits::dynamic_sized_packet::DynamicSizedPacket;

use crate::codec::mpeg4_generic::parameters::RtpMpeg4OutOfBandParams;

use super::{AccessUnit, AccessUnitFragment, AccessUnitSection};

impl DynamicSizedPacket for AccessUnit {
    fn get_packet_bytes_count(&self) -> usize {
        self.body.len()
    }
}

impl DynamicSizedPacket for AccessUnitFragment {
    fn get_packet_bytes_count(&self) -> usize {
        self.body.len()
    }
}
pub struct AccessUnitSectionBytesCountWrapper<'a>(
    pub &'a AccessUnitSection,
    pub &'a RtpMpeg4OutOfBandParams,
);
impl<'a> DynamicSizedPacket for AccessUnitSectionBytesCountWrapper<'a> {
    fn get_packet_bytes_count(&self) -> usize {
        match &self.0.access_units_or_fragment {
            Either::Left(aus) => aus
                .iter()
                .fold(0, |prev, au| au.get_packet_bytes_count() + prev),
            Either::Right(frag) => frag.get_packet_bytes_count(),
        }
    }
}
