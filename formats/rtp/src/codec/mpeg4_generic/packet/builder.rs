use std::{
    cmp,
    io::{self, Read},
};

use num::ToPrimitive;
use tokio_util::{
    bytes::{Buf, Bytes, BytesMut},
    either::Either,
};
use utils::traits::dynamic_sized_packet::{DynamicSizedBitsPacket, DynamicSizedPacket};

use crate::{
    codec::mpeg4_generic::{
        access_unit::{AccessUnit, AccessUnitFragment, AccessUnitSection},
        au_header::{AuHeader, AuHeaderSection, packet_size::AuHeaderBitsCountWrapper},
        errors::{RtpMpeg4Error, RtpMpeg4Result},
        parameters::RtpMpeg4OutOfBandParams,
    },
    header::RtpHeader,
};

use super::RtpMpeg4GenericPacket;

#[derive(Debug)]
pub struct RtpMpeg4GenericPacketBuilder {
    params: RtpMpeg4OutOfBandParams,
    au_index: u64,
    rtp_header: RtpHeader,
    access_units: Vec<Bytes>,
    mtu: u64,
}
const DEFAULT_MTU: u64 = 1400;

impl RtpMpeg4GenericPacketBuilder {
    pub fn new(params: RtpMpeg4OutOfBandParams, mtu: Option<u64>) -> Self {
        Self {
            params,
            au_index: 0,
            rtp_header: Default::default(),
            access_units: vec![],
            mtu: mtu.unwrap_or(DEFAULT_MTU),
        }
    }
    pub fn params(mut self, params: RtpMpeg4OutOfBandParams) -> Self {
        self.params = params;
        self
    }

    pub fn rtp_header(mut self, rtp_header: RtpHeader) -> Self {
        self.rtp_header = rtp_header;
        self
    }

    pub fn access_unit(mut self, access_unit: Bytes) -> Self {
        self.access_units.push(access_unit);
        self
    }

    pub fn mtu(mut self, mtu: u64) -> Self {
        self.mtu = mtu;
        self
    }

    fn packetize_fragmentated(
        &self,
        au_index: &mut u64,
        au: &Bytes,
        timestamp: u32,
    ) -> RtpMpeg4Result<Vec<RtpMpeg4GenericPacket>> {
        if !self.params.allow_fragmentation() {
            return Err(RtpMpeg4Error::PacketizeToRtpFailed(format!(
                "unable to packetize to rtp packet with mtu: {}, au size: {}, rtp header size: {}, and params: {}",
                self.mtu,
                au.len(),
                self.rtp_header.get_packet_bytes_count(),
                self.params
            )));
        }

        let mut reader = io::Cursor::new(au);
        let mut au_header_builder = AuHeader::builder();

        au_header_builder = au_header_builder
            .au_index(Some(*au_index))
            .cts_delta(None)
            .dts_delta(None)
            .stream_state(None)
            .au_size(Some(0))
            .rap_flag(Some(true));

        let mut result = vec![];

        let au_header_bits_cnt = AuHeaderBitsCountWrapper(
            &au_header_builder.build(&self.params, result.is_empty(), true)?,
            &self.params,
        )
        .get_packet_bits_count();

        while reader.has_remaining() {
            let frag_au_size = cmp::min(
                self.mtu as usize
                    - self.rtp_header.get_packet_bytes_count()
                    - au_header_bits_cnt.div_ceil(8),
                reader.remaining(),
            );

            au_header_builder = au_header_builder
                .au_size(Some(frag_au_size as u64))
                .rap_flag(Some(result.is_empty()))
                .au_index(if result.is_empty() {
                    *au_index += 1;
                    Some(*au_index)
                } else {
                    None
                })
                .au_index_delta(if result.is_empty() { None } else { Some(0) });

            let au_header = au_header_builder.build(&self.params, result.is_empty(), true)?;

            let mut frag_bytes = BytesMut::zeroed(frag_au_size);
            reader.read_exact(&mut frag_bytes)?;

            result.push(RtpMpeg4GenericPacket {
                header: self.rtp_header.clone(), // TODO: deal with the seqnum and timestamp
                au_header_section: Some(AuHeaderSection {
                    au_headers: vec![au_header.clone()],
                    au_headers_length: au_header_bits_cnt as u64,
                }),
                auxiliary_data_section: None,
                au_section: AccessUnitSection {
                    access_units_or_fragment: Either::Right(AccessUnitFragment {
                        timestamp,
                        header: au_header,
                        body: frag_bytes,
                    }),
                },
            });
        }

        Ok(result)
    }

    pub fn build(&mut self) -> RtpMpeg4Result<Vec<RtpMpeg4GenericPacket>> {
        let mut result = vec![];

        let fragmented_packet_extra_size = self.rtp_header.get_packet_bytes_count()
            + AuHeaderBitsCountWrapper(
                &AuHeader::builder()
                    .au_index(Some(0))
                    .au_index_delta(Some(0))
                    .au_size(Some(0))
                    .cts_delta(Some(0))
                    .dts_delta(Some(0))
                    .rap_flag(Some(false))
                    .stream_state(Some(0))
                    .build(&self.params, true, true)?,
                &self.params,
            )
            .get_packet_bits_count()
            .div_ceil(8);

        for au in &self.access_units {
            if au.len() + fragmented_packet_extra_size + self.rtp_header.get_packet_bytes_count()
                > self.mtu.to_usize().expect("integer overflow usize")
            {
                let mut au_index = self.au_index;
                // TODO: assume au header takes 10 bytes, this could be optimized
                result.extend(self.packetize_fragmentated(
                    &mut au_index,
                    au,
                    self.rtp_header.timestamp,
                )?);
                self.au_index = au_index;
            } else {
                self.au_index += 1;
                let au_header = AuHeader::builder()
                    .au_index(Some(self.au_index))
                    .au_index_delta(Some(0))
                    .au_size(Some(au.len() as u64))
                    .cts_delta(None)
                    .dts_delta(None)
                    .rap_flag(Some(false))
                    .stream_state(None)
                    .build(&self.params, true, false)?;

                result.push(RtpMpeg4GenericPacket {
                    header: self.rtp_header.clone(),
                    au_header_section: Some(AuHeaderSection {
                        au_headers_length: AuHeaderBitsCountWrapper(&au_header, &self.params)
                            .get_packet_bits_count()
                            as u64,
                        au_headers: vec![au_header.clone()],
                    }),
                    auxiliary_data_section: None,
                    au_section: AccessUnitSection {
                        access_units_or_fragment: Either::Left(vec![AccessUnit {
                            header: au_header,
                            body: au.clone(),
                            timestamp: self.rtp_header.timestamp,
                        }]),
                    },
                });
            }
        }

        Ok(result)
    }
}
