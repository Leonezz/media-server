use std::io::{self, Read};

use utils::traits::reader::ReadRemainingFrom;

use crate::{
    codec::mpeg4_generic::{
        access_unit::AccessUnitSection, au_header::AuHeaderSection, auxiliary::AuxiliaryData,
        errors::RtpMpeg4Error, parameters::RtpMpeg4Fmtp,
    },
    header::RtpHeader,
};

use super::RtpMpeg4GenericPacket;

impl<R: io::Read> ReadRemainingFrom<(&RtpMpeg4Fmtp, &RtpHeader), R>
    for RtpMpeg4GenericPacket
{
    type Error = RtpMpeg4Error;
    fn read_remaining_from(
        header: (&RtpMpeg4Fmtp, &RtpHeader),
        reader: &mut R,
    ) -> Result<Self, Self::Error> {
        let (param, rtp_header) = header;
        let au_header = if !param.guess_has_au_headers() {
            None
        } else {
            Some(AuHeaderSection::read_remaining_from(param, reader)?)
        };
        let auxiliary = if param.auxiliary_data_size_length.is_some() {
            Some(AuxiliaryData::read_remaining_from(param, reader)?)
        } else {
            None
        };

        let mut bytes = vec![];
        reader.read_to_end(&mut bytes)?;
        let in_fragment_mode = !rtp_header.marker;

        let mut reader = io::Cursor::new(bytes);
        let au_section = AccessUnitSection::read_remaining_from(
            (
                au_header
                    .as_ref()
                    .map(|item| item.au_headers.as_ref())
                    .unwrap_or(vec![].as_ref()),
                rtp_header.timestamp,
                in_fragment_mode,
                param,
            ),
            reader.by_ref(),
        )?;
        Ok(Self {
            header: rtp_header.clone(),
            au_header_section: au_header,
            auxiliary_data_section: auxiliary,
            au_section,
        })
    }
}
