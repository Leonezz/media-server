use std::io;

use utils::traits::writer::WriteTo;

use crate::codec::mpeg4_generic::{
    au_header::write::AuHeaderSectionWriteWrapper, auxiliary::write::AuxiliaryDataWriteWrapper,
    errors::RtpMpeg4Error, parameters::RtpMpeg4OutOfBandParams,
};

use super::RtpMpeg4GenericPacket;

pub struct RtpMpeg4GenericPacketWriteWrapper<'a>(
    pub &'a RtpMpeg4GenericPacket,
    pub &'a RtpMpeg4OutOfBandParams,
);
impl<'a, W: io::Write> WriteTo<W> for RtpMpeg4GenericPacketWriteWrapper<'a> {
    type Error = RtpMpeg4Error;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let (packet, params) = (self.0, self.1);
        packet.header.write_to(writer.by_ref())?;
        if let Some(au_header) = packet.au_header_section.as_ref() {
            AuHeaderSectionWriteWrapper(au_header, params).write_to(writer.by_ref())?;
        }
        if let Some(auxiliary) = packet.auxiliary_data_section.as_ref() {
            AuxiliaryDataWriteWrapper(auxiliary, params).write_to(writer.by_ref())?;
        }
        packet.au_section.write_to(writer.by_ref())?;

        Ok(())
    }
}
