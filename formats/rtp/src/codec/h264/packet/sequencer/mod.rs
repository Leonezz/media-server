use super::RtpH264Packet;
use crate::codec::h264::aggregation::{AggregatedPayload, AggregationPacketType};
use crate::codec::h264::packet::sequencer::fragments::RtpH264FragmentsBufferItem;
use crate::codec::h264::packet::sequencer::timestamp_grouper::TimestampGrouper;
use crate::{
    codec::h264::{
        RtpH264NalUnit,
        aggregation::AggregationNalUnits,
        errors::{RtpH264Error, RtpH264Result},
        fragmented::FragmentedUnit,
        paramters::packetization_mode::PacketizationMode,
        single_nalu::SingleNalUnit,
    },
    errors::RtpError,
    header::RtpHeader,
    packet::sequencer::{RtpBufferItem, RtpBufferVideoItem, RtpBufferedSequencer},
};
use codec_h264::{nalu::NalUnit, nalu_type::NALUType, pps::Pps, sps::Sps};
use de_interleaving::{DeInterleavingBuffer, RtpH264DeInterleavingParameters};
use fragments::RtpH264FragmentsBuffer;
use std::collections::VecDeque;
use std::vec;
use utils::traits::buffer::{GenericFragmentComposer, GenericSequencer};
pub mod de_interleaving;
pub mod fragments;
pub mod timestamp_grouper;

#[derive(Debug, Clone)]
pub struct RtpH264BufferItem {
    pub nal_units: Vec<NalUnit>,
    pub is_idr: bool,
    pub sps: Option<NalUnit>,
    pub pps: Option<NalUnit>,
    pub rtp_header: RtpHeader,
    pub decode_order_number: Option<u16>,
    pub timestamp_offset: Option<u32>,
}

impl RtpH264BufferItem {
    pub fn new(
        nal_units: Vec<NalUnit>,
        rtp_header: RtpHeader,
        decode_order_number: Option<u16>,
        timestamp_offset: Option<u32>,
        sps: Option<NalUnit>,
        pps: Option<NalUnit>,
    ) -> Self {
        let is_idr = nal_units
            .iter()
            .any(|nal| nal.header.nal_unit_type == NALUType::IDRSlice);

        Self {
            nal_units,
            is_idr,
            sps: if is_idr { sps } else { None },
            pps: if is_idr { pps } else { None },
            rtp_header: rtp_header.clone(),
            decode_order_number,
            timestamp_offset,
        }
    }

    pub fn merge(&mut self, other: Self) {
        assert_eq!(self.rtp_header.timestamp, other.rtp_header.timestamp);
        assert_eq!(self.timestamp_offset, other.timestamp_offset);
        assert_eq!(self.decode_order_number, other.decode_order_number);

        self.nal_units.extend(other.nal_units);
        self.is_idr = self
            .nal_units
            .iter()
            .any(|nal| matches!(nal.header.nal_unit_type, NALUType::IDRSlice));
        if self.sps.is_none() {
            self.sps = other.sps;
        }
        if self.pps.is_none() {
            self.pps = other.pps;
        }
    }
}

const DEFAULT_BUFFER_CAPACITY: usize = 1000 * 1000;
const DEFAULT_FRAGMENT_BUFFER_CAPACITY: usize = 500000;

pub struct RtpH264Sequencer {
    buffer_capacity: usize,
    decode_order_number_cycles: usize,
    packetization_mode: PacketizationMode,
    decoder_buffer: VecDeque<RtpH264BufferItem>,
    de_interleaving_buffer: Option<DeInterleavingBuffer>,
    fragments_buffer: Option<RtpH264FragmentsBuffer>,
    timestamp_grouper: Option<TimestampGrouper>,
    sps: Option<NalUnit>,
    pps: Option<NalUnit>,
}

impl RtpH264Sequencer {
    pub fn new(
        packetization_mode: PacketizationMode,
        de_interleaving_parameters: RtpH264DeInterleavingParameters,
        initial_sps: Option<Sps>,
        initial_pps: Option<Pps>,
    ) -> Self {
        tracing::info!(
            "creating h264 rtp sequencer with: {}, {:?}",
            packetization_mode,
            de_interleaving_parameters
        );
        Self {
            buffer_capacity: DEFAULT_BUFFER_CAPACITY,
            fragments_buffer: Some(RtpH264FragmentsBuffer::new(
                DEFAULT_FRAGMENT_BUFFER_CAPACITY,
            )),
            decode_order_number_cycles: 0,

            packetization_mode,

            decoder_buffer: VecDeque::with_capacity(DEFAULT_BUFFER_CAPACITY),
            de_interleaving_buffer: if packetization_mode == PacketizationMode::Interleaved {
                Some(DeInterleavingBuffer::new(de_interleaving_parameters))
            } else {
                None
            },
            timestamp_grouper: Some(TimestampGrouper::new()),
            sps: initial_sps.map(|v| (&v).into()),
            pps: initial_pps.map(|v| (&v).into()),
        }
    }

    fn enqueue_decoder_buffer(&mut self, mut item: RtpH264BufferItem) -> RtpH264Result<()> {
        if let Some(timestamp_grouper) = self.timestamp_grouper.as_mut() {
            let groupped = timestamp_grouper.enqueue(item)?;
            if groupped.is_none() {
                return Ok(());
            }
            item = groupped.unwrap();
        }

        if self.decoder_buffer.len() >= self.buffer_capacity {
            let dropped = self.decoder_buffer.pop_front();
            tracing::warn!(
                "dropped item from rtp h264 sequencer because of capacity exceed: {:?}",
                dropped
            );
        }

        for nal in &item.nal_units {
            if nal.header.nal_unit_type == NALUType::SPS {
                self.sps = Some(nal.clone());
            } else if nal.header.nal_unit_type == NALUType::PPS {
                self.pps = Some(nal.clone());
            } else if matches!(nal.header.nal_unit_type, NALUType::IDRSlice) {
                item.sps = self.sps.clone();
                item.pps = self.pps.clone();
            }
        }

        self.decoder_buffer.push_back(item);

        Ok(())
    }

    fn enqueue_de_interleaving_buffer(&mut self, item: RtpH264BufferItem) -> RtpH264Result<()> {
        if let Some(de_interleaving) = &mut self.de_interleaving_buffer {
            de_interleaving.enqueue(item).unwrap();
            de_interleaving
                .try_dump()
                .into_iter()
                .try_for_each(|item| self.enqueue_decoder_buffer(item))?;
        }

        Ok(())
    }

    fn on_aggregated_packet(
        &mut self,
        rtp_header: RtpHeader,
        packet: AggregationNalUnits,
    ) -> RtpH264Result<()> {
        let _span = tracing::debug_span!(
            "on_aggregated_packet",
            rtp_sequence_number = rtp_header.sequence_number,
            rtp_timestamp = rtp_header.timestamp,
            rtp_payload_type = rtp_header.payload_type,
        )
        .entered();
        match packet.payload {
            AggregatedPayload::StapA(stap_a_packet) => {
                let item = RtpH264BufferItem::new(
                    stap_a_packet.nal_units,
                    rtp_header,
                    None,
                    None,
                    None,
                    None,
                );
                self.enqueue_decoder_buffer(item)?;
            }
            AggregatedPayload::StapB(stap_b_packet) => {
                let item = RtpH264BufferItem::new(
                    stap_b_packet.nal_units,
                    rtp_header,
                    Some(stap_b_packet.decode_order_number),
                    None,
                    None,
                    None,
                );
                self.enqueue_de_interleaving_buffer(item)?;
            }
            AggregatedPayload::Mtap16(mtap16_packet) => {
                mtap16_packet.nal_units.into_iter().try_for_each(|item| {
                    if mtap16_packet
                        .decode_order_number_base
                        .checked_add(item.decode_order_number_diff as u16)
                        .is_none()
                    {
                        self.decode_order_number_cycles += 1;
                    }

                    let item = RtpH264BufferItem::new(
                        vec![item.nal_unit],
                        rtp_header.clone(),
                        Some(
                            mtap16_packet
                                .decode_order_number_base
                                .wrapping_add(item.decode_order_number_diff as u16),
                        ),
                        Some(item.timestamp_offset as u32),
                        None,
                        None,
                    );
                    self.enqueue_de_interleaving_buffer(item)
                })?;
            }
            AggregatedPayload::Mtap24(mtap24_packet) => {
                mtap24_packet.nal_units.into_iter().try_for_each(|item| {
                    if mtap24_packet
                        .decode_order_number_base
                        .checked_add(item.decode_order_number_diff as u16)
                        .is_none()
                    {
                        self.decode_order_number_cycles += 1;
                    }

                    let item = RtpH264BufferItem::new(
                        vec![item.nal_unit],
                        rtp_header.clone(),
                        Some(
                            mtap24_packet
                                .decode_order_number_base
                                .wrapping_add(item.decode_order_number_diff as u16),
                        ),
                        Some(item.timestamp_offset),
                        None,
                        None,
                    );
                    self.enqueue_de_interleaving_buffer(item)
                })?;
            }
        }
        Ok(())
    }

    fn on_fragmented(
        &mut self,
        rtp_header: RtpHeader,
        packet: FragmentedUnit,
    ) -> RtpH264Result<()> {
        if let Some(fragment_buffer) = &mut self.fragments_buffer {
            let fragment_item = RtpH264FragmentsBufferItem {
                rtp_header,
                fragment: packet,
            };
            let packet = fragment_buffer.enqueue(fragment_item)?;
            if let Some(packet) = packet {
                if packet.decode_order_number.is_some() {
                    self.enqueue_de_interleaving_buffer(packet)?;
                } else {
                    self.enqueue_decoder_buffer(packet)?;
                }
            }
        }

        Ok(())
    }

    fn check_packet(&self, packet: &RtpH264Packet) -> RtpH264Result<()> {
        match &packet.payload {
            RtpH264NalUnit::SingleNalu(_) => {
                if self.packetization_mode == PacketizationMode::Interleaved {
                    return Err(RtpH264Error::UnexpectedPacketType(format!(
                        "got single nalu packet while in Interleaved packetization mode, header: {:?}",
                        packet.header
                    )));
                }
            }
            RtpH264NalUnit::Aggregated(aggregated) => match &aggregated.header.aggregate_type {
                AggregationPacketType::MTAP16 | AggregationPacketType::MTAP24 => {
                    if self.packetization_mode != PacketizationMode::Interleaved {
                        return Err(RtpH264Error::UnexpectedPacketType(format!(
                            "got mtap packet while not in Interleaved packetization mode, mode: {}, header: {:?}",
                            self.packetization_mode, packet.header
                        )));
                    }
                }
                AggregationPacketType::STAPA => {
                    if self.packetization_mode != PacketizationMode::NonInterleaved {
                        return Err(RtpH264Error::UnexpectedPacketType(format!(
                            "got stap-A packet while not in NonInterleaved mode, mode: {}, header: {:?}",
                            self.packetization_mode, packet.header
                        )));
                    }
                }
                AggregationPacketType::STAPB => {
                    if self.packetization_mode != PacketizationMode::Interleaved {
                        return Err(RtpH264Error::UnexpectedPacketType(format!(
                            "got stap-B packet while not in Interleaved mode, mode: {}, header: {:?}",
                            self.packetization_mode, packet.header
                        )));
                    }
                }
            },
            RtpH264NalUnit::Fragmented(fragmented) => match &fragmented {
                FragmentedUnit::FuA(_) => {
                    if self.packetization_mode == PacketizationMode::SingleNalu {
                        return Err(RtpH264Error::UnexpectedPacketType(format!(
                            "got fu-A packet while in SingleNalu mode, header: {:?}",
                            packet.header
                        )));
                    }
                }
                FragmentedUnit::FuB(_) => {
                    if self.packetization_mode != PacketizationMode::Interleaved {
                        return Err(RtpH264Error::UnexpectedPacketType(format!(
                            "got fu-B packet while not in Interleaved mode, mode: {}, header: {:?}",
                            self.packetization_mode, packet.header
                        )));
                    }
                }
            },
        }
        Ok(())
    }

    pub fn on_packet(&mut self, packet: RtpH264Packet) -> RtpH264Result<()> {
        self.check_packet(&packet)?;
        let rtp_header = packet.header;
        match packet.payload {
            RtpH264NalUnit::SingleNalu(SingleNalUnit(nalu)) => {
                let item = RtpH264BufferItem::new(vec![nalu], rtp_header, None, None, None, None);
                self.enqueue_decoder_buffer(item)?;
            }
            RtpH264NalUnit::Aggregated(aggregation) => {
                self.on_aggregated_packet(rtp_header, aggregation)?;
            }
            RtpH264NalUnit::Fragmented(fragmentation) => {
                self.on_fragmented(rtp_header, fragmentation)?
            }
        }

        Ok(())
    }

    pub fn try_dump_packets(&mut self) -> Vec<RtpH264BufferItem> {
        let mut result: Vec<RtpH264BufferItem> = Vec::with_capacity(self.decoder_buffer.len());
        while let Some(item) = self.decoder_buffer.pop_front() {
            result.push(item);
        }
        result
    }
}

impl RtpBufferedSequencer for RtpH264Sequencer {
    fn enqueue(&mut self, packet: crate::packet::RtpTrivialPacket) -> Result<(), RtpError> {
        let h264_packet: RtpH264Packet = packet
            .try_into()
            .map_err(|err| RtpError::H264SequenceFailed(format!("{}", err)))?;

        self.on_packet(h264_packet)
            .map_err(|err| RtpError::H264SequenceFailed(format!("{}", err)))?;
        Ok(())
    }

    fn try_dump(&mut self) -> Vec<crate::packet::sequencer::RtpBufferItem> {
        let packets = self.try_dump_packets();
        packets
            .into_iter()
            .map(|item| RtpBufferItem::Video(RtpBufferVideoItem::H264(item)))
            .collect()
    }
}
