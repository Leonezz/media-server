use std::collections::VecDeque;

use codec_h264::nalu::NalUnit;
use de_interleaving::{DeInterleavingBuffer, RtpH264DeInterleavingParameters};
use fragments::RtpH264FragmentsBuffer;

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
    packet::sequencer::{
        GenericFragmentComposer, GenericSequencer, RtpBufferItem, RtpBufferVideoItem,
        RtpBufferedSequencer,
    },
};

use super::RtpH264Packet;
pub mod de_interleaving;
pub mod fragments;

#[derive(Debug, Clone)]
pub struct RtpH264BufferItem {
    pub nal_unit: NalUnit,
    pub rtp_header: RtpHeader,
    pub decode_order_number: Option<u16>,
    pub timestamp_offset: Option<u32>,
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
}

impl RtpH264Sequencer {
    pub fn new(
        packetization_mode: PacketizationMode,
        de_interleaving_parameters: RtpH264DeInterleavingParameters,
    ) -> Self {
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
        }
    }

    fn enqueue_decoder_buffer(&mut self, item: RtpH264BufferItem) -> RtpH264Result<()> {
        if self.decoder_buffer.len() >= self.buffer_capacity {
            let dropped = self.decoder_buffer.pop_front();
            tracing::warn!("dropped item from rtp h264 sequencer: {:?}", dropped);
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
                .try_for_each(|item| {
                    self.enqueue_decoder_buffer(RtpH264BufferItem {
                        nal_unit: item.nal_unit,
                        rtp_header: item.rtp_header,
                        decode_order_number: Some(item.decode_order_number.unwrap()),
                        timestamp_offset: item.timestamp_offset,
                    })
                })?;
        }

        Ok(())
    }

    fn on_aggregated_packet(
        &mut self,
        rtp_header: RtpHeader,
        packet: AggregationNalUnits,
    ) -> RtpH264Result<()> {
        match packet {
            AggregationNalUnits::StapA(stap_a_packet) => {
                stap_a_packet.nal_units.into_iter().try_for_each(|item| {
                    self.enqueue_decoder_buffer(RtpH264BufferItem {
                        nal_unit: item,
                        rtp_header: rtp_header.clone(),
                        decode_order_number: None,
                        timestamp_offset: None,
                    })
                })?;
            }
            AggregationNalUnits::StapB(stap_b_packet) => {
                stap_b_packet.nal_units.into_iter().try_for_each(|item| {
                    self.enqueue_de_interleaving_buffer(RtpH264BufferItem {
                        nal_unit: item,
                        rtp_header: rtp_header.clone(),
                        decode_order_number: Some(stap_b_packet.decode_order_number),
                        timestamp_offset: None,
                    })
                })?;
            }
            AggregationNalUnits::Mtap16(mtap16_packet) => {
                mtap16_packet.nal_units.into_iter().try_for_each(|item| {
                    if mtap16_packet
                        .decode_order_number_base
                        .checked_add(item.decode_order_number_diff as u16)
                        .is_none()
                    {
                        self.decode_order_number_cycles += 1;
                    }

                    self.enqueue_de_interleaving_buffer(RtpH264BufferItem {
                        nal_unit: item.nal_unit,
                        rtp_header: rtp_header.clone(),
                        decode_order_number: Some(
                            mtap16_packet
                                .decode_order_number_base
                                .wrapping_add(item.decode_order_number_diff as u16),
                        ),
                        timestamp_offset: Some(item.timestamp_offset as u32),
                    })
                })?;
            }
            AggregationNalUnits::Mtap24(mtap24_packet) => {
                mtap24_packet.nal_units.into_iter().try_for_each(|item| {
                    if mtap24_packet
                        .decode_order_number_base
                        .checked_add(item.decode_order_number_diff as u16)
                        .is_none()
                    {
                        self.decode_order_number_cycles += 1;
                    }

                    self.enqueue_de_interleaving_buffer(RtpH264BufferItem {
                        nal_unit: item.nal_unit,
                        rtp_header: rtp_header.clone(),
                        decode_order_number: Some(
                            mtap24_packet
                                .decode_order_number_base
                                .wrapping_add(item.decode_order_number_diff as u16),
                        ),
                        timestamp_offset: Some(item.timestamp_offset),
                    })
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
            let packet = fragment_buffer.enqueue((rtp_header, packet))?;
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
            RtpH264NalUnit::Aggregated(aggregated) => match &aggregated {
                AggregationNalUnits::Mtap16(_) | AggregationNalUnits::Mtap24(_) => {
                    if self.packetization_mode != PacketizationMode::Interleaved {
                        return Err(RtpH264Error::UnexpectedPacketType(format!(
                            "got mtap packet while not in Interleaved packetization mode, mode: {}, header: {:?}",
                            self.packetization_mode, packet.header
                        )));
                    }
                }
                AggregationNalUnits::StapA(_) => {
                    if self.packetization_mode != PacketizationMode::NonInterleaved {
                        return Err(RtpH264Error::UnexpectedPacketType(format!(
                            "got stap-A packet while not in NonInterleaved mode, mode: {}, header: {:?}",
                            self.packetization_mode, packet.header
                        )));
                    }
                }
                AggregationNalUnits::StapB(_) => {
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
                self.enqueue_decoder_buffer(RtpH264BufferItem {
                    nal_unit: nalu,
                    rtp_header,
                    decode_order_number: None,
                    timestamp_offset: None,
                })?;
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
