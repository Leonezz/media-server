pub mod packet_size;
pub mod packetizer;
pub mod reader;
pub mod sequencer;
pub mod writer;
use tokio_util::bytes::{Buf, Bytes};
use utils::traits::{reader::ReadRemainingFrom, writer::WriteTo};

use super::{
    access_unit::AccessUnitSection, au_header::AuHeaderSection, auxiliary::AuxiliaryData,
    errors::RtpMpeg4Error, parameters::RtpMpeg4Fmtp,
};
use crate::{
    codec::mpeg4_generic::{
        au_header::writer::AuHeaderSectionWriteWrapper,
        auxiliary::writer::AuxiliaryDataWriteWrapper,
    },
    header::RtpHeader,
    packet::RtpTrivialPacket,
};

#[derive(Debug)]
pub struct RtpMpeg4GenericPacket {
    pub header: RtpHeader,
    pub au_header_section: Option<AuHeaderSection>,
    pub auxiliary_data_section: Option<AuxiliaryData>,
    pub au_section: AccessUnitSection,
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

impl TryFrom<(RtpMpeg4GenericPacket, &RtpMpeg4Fmtp)> for RtpTrivialPacket {
    type Error = RtpMpeg4Error;
    fn try_from(value: (RtpMpeg4GenericPacket, &RtpMpeg4Fmtp)) -> Result<Self, Self::Error> {
        let mut payload = Vec::with_capacity(1500);
        let (packet, params) = value;
        if let Some(au_header) = packet.au_header_section.as_ref() {
            AuHeaderSectionWriteWrapper(au_header, params).write_to(&mut payload);
        }
        if let Some(auxiliary) = packet.auxiliary_data_section.as_ref() {
            AuxiliaryDataWriteWrapper(auxiliary, params).write_to(&mut payload)?;
        }
        packet.au_section.write_to(&mut payload)?;
        Ok(Self {
            header: packet.header,
            payload: Bytes::from_owner(payload),
        })
    }
}
