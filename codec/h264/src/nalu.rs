use std::fmt;

use tokio_util::bytes::Bytes;
use utils::traits::{dynamic_sized_packet::DynamicSizedPacket, fixed_packet::FixedPacket};

use crate::nalu_header::NaluHeader;

#[derive(Clone)]
pub struct NalUnit {
    pub header: NaluHeader,
    // bytes in body does not include the header byte
    pub body: Bytes,
}

impl fmt::Debug for NalUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "nal_header: {:?}, payload length: {}",
            self.header,
            self.body.len()
        )
    }
}

impl DynamicSizedPacket for NalUnit {
    fn get_packet_bytes_count(&self) -> usize {
        NaluHeader::bytes_count() + self.body.len()
    }
}
