use super::RtpMpeg4GenericPacket;
use crate::{
    codec::mpeg4_generic::{
        access_unit::{AccessUnit, AccessUnitFragment, AccessUnitSection},
        au_header::{AuHeader, AuHeaderSection, packet_size::AuHeaderBitsCountWrapper},
        errors::{RtpMpeg4Error, RtpMpeg4Result},
        parameters::RtpMpeg4Fmtp,
    },
    errors::RtpError,
    header::{RtpHeader, RtpHeaderBuilder},
    packet::packetizer::{RtpTrivialPacketPacketizer, wallclock_to_rtp_timestamp},
    payload_types::rtp_payload_type::{audio_get_rtp_clockrate, get_audio_rtp_payload_type},
};
use codec_common::audio::AudioCodecCommon;
use num::ToPrimitive;
use std::{
    cmp,
    io::{self, Read},
};
use tokio_util::{
    bytes::{Buf, Bytes, BytesMut},
    either::Either,
};
use utils::{
    random,
    traits::dynamic_sized_packet::{DynamicSizedBitsPacket, DynamicSizedPacket},
};

#[derive(Debug)]
pub struct RtpMpeg4GenericPacketPacketizer {
    params: RtpMpeg4Fmtp,
    au_index: u64,
    rtp_header: RtpHeader,
    rtp_clockrate: u64,
    first_frame_timestamp: Option<u64>,
    last_frame_timestamp: Option<u64>,
    rtp_timestamp_base: u64,
    access_units: Vec<Bytes>,
    mtu: usize,
}

impl RtpMpeg4GenericPacketPacketizer {
    pub fn new(mtu: usize, params: RtpMpeg4Fmtp, ssrc: u32) -> Self {
        Self {
            params,
            au_index: 0,
            rtp_header: RtpHeaderBuilder::new()
                .version(2)
                .payload_type(get_audio_rtp_payload_type(AudioCodecCommon::AAC).unwrap())
                .ssrc(ssrc)
                .sequence_number(random::random_u16())
                .build(),
            access_units: vec![],
            rtp_clockrate: audio_get_rtp_clockrate(AudioCodecCommon::AAC).unwrap(),
            first_frame_timestamp: None,
            last_frame_timestamp: None,
            rtp_timestamp_base: random::random_u32() as u64,
            mtu,
        }
    }
    pub fn params(&mut self, params: RtpMpeg4Fmtp) -> &mut Self {
        self.params = params;
        self
    }

    pub fn mtu(&mut self, mtu: usize) -> &mut Self {
        self.mtu = mtu;
        self
    }

    fn packetize_fragmentated(
        &mut self,
        au_index: &mut u64,
        au: Bytes,
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

        au_header_builder
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
                self.mtu
                    - self.rtp_header.get_packet_bytes_count()
                    - au_header_bits_cnt.div_ceil(8),
                reader.remaining(),
            );

            au_header_builder
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
            self.rtp_header.sequence_number = self.rtp_header.sequence_number.wrapping_add(1);
        }

        Ok(result)
    }
}

impl RtpTrivialPacketPacketizer for RtpMpeg4GenericPacketPacketizer {
    fn build(&mut self) -> Result<Vec<crate::packet::RtpTrivialPacket>, crate::errors::RtpError> {
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
                    .build(&self.params, true, true)
                    .map_err(|e| RtpError::Mpeg4PacketizationFailed(format!("{}", e)))?,
                &self.params,
            )
            .get_packet_bits_count()
            .div_ceil(8);

        self.au_index = 0;
        let access_units: Vec<_> = self.access_units.drain(..).collect();
        for au in access_units {
            if au.len() + fragmented_packet_extra_size + self.rtp_header.get_packet_bytes_count()
                > self.mtu.to_usize().expect("integer overflow usize")
            {
                let mut au_index = self.au_index;
                // TODO: assume au header takes 10 bytes, this could be optimized
                result.extend(
                    self.packetize_fragmentated(&mut au_index, au, self.rtp_header.timestamp)
                        .map_err(|e| RtpError::Mpeg4PacketizationFailed(format!("{}", e)))?,
                );
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
                    .build(&self.params, true, false)
                    .map_err(|e| RtpError::Mpeg4PacketizationFailed(format!("{}", e)))?;

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

        let mut trivial_packets = Vec::with_capacity(result.len());
        let mut rtp_timestamp_delta = 0;
        for pkt in result {
            let mut trivial_packet: crate::packet::RtpTrivialPacket =
                (pkt, &self.params).try_into().map_err(|e| {
                    RtpError::Mpeg4PacketizationFailed(format!(
                        "unable to convert to rtp packet: {}",
                        e
                    ))
                })?;
            trivial_packet.header.timestamp = wallclock_to_rtp_timestamp(
                self.last_frame_timestamp.unwrap(),
                self.first_frame_timestamp.unwrap(),
                self.rtp_timestamp_base,
                self.rtp_clockrate,
            ) as u32
                + rtp_timestamp_delta;
            rtp_timestamp_delta += 1024;
            trivial_packet.header.sequence_number = self.rtp_header.sequence_number;
            self.rtp_header.sequence_number = self.rtp_header.sequence_number.wrapping_add(1);
            trivial_packet.header.marker = true;
            trivial_packets.push(trivial_packet);
        }
        Ok(trivial_packets)
    }

    fn packetize(
        &mut self,
        item: crate::packet::packetizer::RtpPacketizerItem,
    ) -> Result<(), RtpError> {
        match item {
            crate::packet::packetizer::RtpPacketizerItem::Audio(audio_item) => match audio_item {
                crate::packet::packetizer::RtpPacketizerAudioItem::AAC(mpeg4_item) => {
                    self.access_units.extend(mpeg4_item.access_units);
                    Ok(())
                }
            },
            _ => {
                debug_assert!(false, "only mpeg4 audio item is supported");
                Err(RtpError::Mpeg4PacketizationFailed(
                    "only mpeg4 audio item is supported".to_string(),
                ))
            }
        }
    }

    fn set_rtp_header(&mut self, mut header: RtpHeader) {
        self.au_index = 0;
        header.sequence_number = self.rtp_header.sequence_number;
        self.rtp_header = header;
    }

    fn set_frame_timestamp(&mut self, timestamp: u64) {
        self.au_index = 0;
        self.last_frame_timestamp = Some(timestamp);
        if self.first_frame_timestamp.is_none() {
            self.first_frame_timestamp = Some(timestamp);
        }
    }

    fn get_rtp_clockrate(&self) -> u64 {
        self.rtp_clockrate
    }

    fn rtp_header(&self) -> &RtpHeader {
        &self.rtp_header
    }
}
