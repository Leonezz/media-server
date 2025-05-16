use de_interleaving::RtpMpeg4GenericDeInterleavingBuffer;
use fragments::RtpMpeg4GenericFragmentationBuffer;
use tokio_util::{bytes::Buf, either::Either};
use utils::traits::reader::ReadRemainingFrom;

use crate::{
    codec::mpeg4_generic::{
        access_unit::{AccessUnit, AccessUnitFragment},
        errors::{RtpMpeg4Error, RtpMpeg4Result},
        parameters::RtpMpeg4OutOfBandParams,
    },
    errors::RtpError,
    header::RtpHeader,
    packet::sequencer::{
        GenericFragmentComposer, GenericSequencer, RtpBufferAudioItem, RtpBufferItem,
        RtpBufferedSequencer,
    },
};

use super::RtpMpeg4GenericPacket;

pub mod de_interleaving;
pub mod fragments;

#[derive(Debug)]
pub struct RtpMpeg4GenericBufferItem {
    access_unit: AccessUnit,
    rtp_header: RtpHeader,
}

pub struct RtpMpeg4GenericSequencer {
    params: RtpMpeg4OutOfBandParams,
    de_interleaving_buffer: RtpMpeg4GenericDeInterleavingBuffer,
    fragmentation_buffer: Option<RtpMpeg4GenericFragmentationBuffer>,
}

impl RtpMpeg4GenericSequencer {
    pub fn new(
        param: RtpMpeg4OutOfBandParams,
        capacity: usize,
        initial_buffer_size: usize,
    ) -> Self {
        let de_interleaving_buffer = RtpMpeg4GenericDeInterleavingBuffer::new(
            capacity,
            param.max_displacement.unwrap_or(1000),
            initial_buffer_size,
        );
        let fragmentation_buffer = if param.allow_fragmentation() {
            Some(RtpMpeg4GenericFragmentationBuffer::new(capacity))
        } else {
            None
        };
        Self {
            params: param,
            de_interleaving_buffer,
            fragmentation_buffer,
        }
    }

    fn on_fragmented(
        &mut self,
        rtp_header: RtpHeader,
        fragment: AccessUnitFragment,
    ) -> RtpMpeg4Result<()> {
        if let Some(fragment_buffer) = &mut self.fragmentation_buffer {
            let packet = fragment_buffer.enqueue((rtp_header, fragment))?;
            if let Some(packet) = packet {
                self.on_access_units(packet.rtp_header, vec![packet.access_unit])?;
            }
        } else {
            tracing::error!(
                "got fragment packet while fragmentation is not enabled: {}",
                self.params
            );
            return Err(RtpMpeg4Error::UnexpectedFragmentPacket(format!(
                "got fragment packet while fragmentation is not enabled: {}",
                self.params
            )));
        }

        Ok(())
    }

    fn on_access_units(
        &mut self,
        rtp_header: RtpHeader,
        access_units: Vec<AccessUnit>,
    ) -> RtpMpeg4Result<()> {
        let to_buffer_items = || -> Vec<_> {
            access_units
                .into_iter()
                .map(|item| RtpMpeg4GenericBufferItem {
                    rtp_header: rtp_header.clone(),
                    access_unit: item,
                })
                .collect()
        };
        self.de_interleaving_buffer.enqueue(to_buffer_items())?;
        Ok(())
    }
}

impl RtpBufferedSequencer for RtpMpeg4GenericSequencer {
    fn enqueue(
        &mut self,
        packet: crate::packet::RtpTrivialPacket,
    ) -> Result<(), crate::errors::RtpError> {
        let packet = RtpMpeg4GenericPacket::read_remaining_from(
            (&self.params, &packet.header),
            &mut packet.payload.reader(),
        )
        .map_err(|err| RtpError::Mpeg4SequenceFailed(format!("{}", err)))?;

        let au_headers = packet
            .au_header_section
            .map(|item| item.au_headers)
            .unwrap_or(vec![]);
        if au_headers.is_empty() {
            return Err(RtpError::Mpeg4SequenceFailed(
                "invalid mpeg4 rtp packet: no au header found".to_owned(),
            ));
        }

        match packet.au_section.access_units_or_fragment {
            Either::Left(aus) => {
                if aus.len() != au_headers.len() {
                    return Err(RtpError::Mpeg4SequenceFailed(format!(
                        "au headers count {} and au count {} mismatch",
                        au_headers.len(),
                        aus.len()
                    )));
                }
                self.on_access_units(packet.header, aus)
                    .map_err(|err| RtpError::Mpeg4SequenceFailed(format!("{}", err)))?;
            }
            Either::Right(frag) => {
                if au_headers.len() != 1 {
                    return Err(RtpError::Mpeg4SequenceFailed(format!(
                        "{} au headers found for fragment packet",
                        au_headers.len()
                    )));
                }
                self.on_fragmented(packet.header, frag)
                    .map_err(|err| RtpError::Mpeg4SequenceFailed(format!("{}", err)))?;
            }
        }

        Ok(())
    }

    fn try_dump(&mut self) -> Vec<crate::packet::sequencer::RtpBufferItem> {
        self.de_interleaving_buffer
            .try_dump()
            .into_iter()
            .map(|item| RtpBufferItem::Audio(RtpBufferAudioItem::AAC(item)))
            .collect()
    }
}
