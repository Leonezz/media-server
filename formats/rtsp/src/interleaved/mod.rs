use tokio_util::bytes::Bytes;
use utils::traits::dynamic_sized_packet::DynamicSizedPacket;
pub mod builder;
pub mod framed;
pub mod reader;
pub mod writer;

pub(crate) const DOLLAR_SIGN: u8 = 0x24;

///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |   "$" = 36    |   Channel ID  |        Length in octets       |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// :         Binary data (Length according to Length field)        :
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///
#[derive(Debug, Clone)]
pub struct RtspInterleavedPacket {
    pub channel_id: u8,
    pub payload: Bytes,
}

impl RtspInterleavedPacket {
    pub fn builder() -> builder::RtspInterleavedPacketBuilder {
        builder::RtspInterleavedPacketBuilder::new()
    }
}

impl DynamicSizedPacket for RtspInterleavedPacket {
    fn get_packet_bytes_count(&self) -> usize {
        4 + self.payload.len()
    }
}
