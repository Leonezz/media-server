pub mod builder;
pub mod packet_size;
pub mod reader;
pub mod sequencer;
pub mod writer;

use builder::RtpMpeg4GenericPacketBuilder;
use tokio_util::bytes::Buf;
use utils::traits::reader::ReadRemainingFrom;

use crate::{header::RtpHeader, packet::RtpTrivialPacket};

use super::{
    access_unit::AccessUnitSection, au_header::AuHeaderSection, auxiliary::AuxiliaryData,
    errors::RtpMpeg4Error, parameters::RtpMpeg4Fmtp,
};

#[derive(Debug)]
pub struct RtpMpeg4GenericPacket {
    pub header: RtpHeader,
    pub au_header_section: Option<AuHeaderSection>,
    pub auxiliary_data_section: Option<AuxiliaryData>,
    pub au_section: AccessUnitSection,
}

impl RtpMpeg4GenericPacket {
    pub fn builder(
        param: RtpMpeg4Fmtp,
        mtu: Option<u64>,
    ) -> RtpMpeg4GenericPacketBuilder {
        RtpMpeg4GenericPacketBuilder::new(param, mtu)
    }
}

impl TryFrom<(RtpTrivialPacket, &RtpMpeg4Fmtp)> for RtpMpeg4GenericPacket {
    type Error = RtpMpeg4Error;
    fn try_from(value: (RtpTrivialPacket, &RtpMpeg4Fmtp)) -> Result<Self, Self::Error> {
        let (rtp_trivial_packet, params) = value;
        Self::read_remaining_from(
            (params, &rtp_trivial_packet.header),
            &mut rtp_trivial_packet.payload.reader(),
        )
    }
}
