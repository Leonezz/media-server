use std::{
    collections::VecDeque,
    io::Cursor,
    time::{self},
};

use h264_codec::nalu::NalUnit;
use tokio_util::bytes::BytesMut;
use utils::traits::reader::ReadFrom;

use crate::{
    codec::h264::{
        RtpH264NalUnit,
        aggregation::AggregationNalUnits,
        errors::{RtpH264Error, RtpH264Result},
        fragmented::FragmentedUnit,
        paramters::packetization_mode::PacketizationMode,
        single_nalu::SingleNalUnit,
        util::don_diff,
    },
    header::RtpHeader,
};

use super::RtpH264Packet;

#[derive(Debug, Clone)]
pub struct RtpH264BufferItem {
    pub nal_unit: NalUnit,
    pub rtp_header: RtpHeader,
    pub decode_order_number: Option<u16>,
    pub timestamp_offset: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct DeInterleavingBufferItem {
    pub nal_unit: NalUnit,
    pub rtp_header: RtpHeader,
    pub decode_order_number: u16,
    pub timestamp_offset: Option<u32>,
}

const DEFAULT_BUFFER_CAPACITY: usize = 1000 * 1000;
const DEFAULT_FRAGMENT_BUFFER_CAPACITY: usize = 50000;

#[derive(Debug, Default)]
pub struct DeInterleavingParameters {
    pub sprop_interleaving_depth: Option<u64>,
    pub sprop_deint_buf_req: Option<u64>,
    pub sprop_init_buf_time: Option<u64>,
    pub sprop_max_don_diff: Option<u64>,
}

#[derive(Debug)]
pub struct DeInterleavingBuffer {
    parameters: DeInterleavingParameters,

    initial_buffering_until: Option<time::Instant>,
    pdon: u64,
    buffer: VecDeque<(u64, DeInterleavingBufferItem)>,
}
impl DeInterleavingBuffer {
    pub fn new(parameters: DeInterleavingParameters) -> Self {
        let deint_buf_size_req = parameters
            .sprop_deint_buf_req
            .unwrap_or(DEFAULT_BUFFER_CAPACITY as u64);
        let initial_buf_time = parameters.sprop_init_buf_time;
        Self {
            parameters,
            initial_buffering_until: initial_buf_time.map(|time| {
                time::Instant::now() + time::Duration::from_millis(time / 9_0000 * 1000)
            }),
            pdon: 0,
            buffer: VecDeque::with_capacity(deint_buf_size_req as usize / 1000 / 10),
        }
    }
    pub fn calculate_abs_don(&mut self) {
        if self.buffer.is_empty() {
            return;
        }
        let mut prev_don = self.buffer[0].1.decode_order_number as u64;
        let mut prev_abs_don = prev_don;
        for (abs_don, item) in &mut self.buffer {
            let current_don = item.decode_order_number as u64;
            if prev_don == current_don {
                *abs_don = prev_abs_don;
            } else if (prev_don < current_don) && (current_don - prev_don) < 32768 {
                *abs_don = prev_abs_don + (current_don - prev_don);
            } else if (prev_don > current_don) && (prev_don - current_don) >= 32768 {
                *abs_don = prev_abs_don + 65536 - prev_don + current_don;
            } else if (prev_don < current_don) && (current_don - prev_don) >= 32768 {
                *abs_don = prev_abs_don - (prev_don + 65536 - current_don);
            } else if (prev_don > current_don) && (prev_don - current_don) < 32768 {
                *abs_don = prev_abs_don - (prev_don - current_don);
            }

            prev_don = current_don;
            prev_abs_don = *abs_don;
        }
    }

    pub fn push_back(&mut self, item: DeInterleavingBufferItem) {
        self.buffer.push_back((0, item));
        self.calculate_abs_don();
    }

    fn try_pop_one(&mut self) -> Option<DeInterleavingBufferItem> {
        if self.buffer.is_empty() {
            return None;
        }

        let mut prev_index = 0;
        let mut prev_min_distance = u64::MAX;
        for (index, (_, item)) in self.buffer.iter().enumerate() {
            let distance = self.pdon_distance(item);
            if distance < prev_min_distance {
                prev_min_distance = distance;
                prev_index = index;
            }
        }

        let item = self.buffer.remove(prev_index).map(|(_, item)| item);
        self.calculate_abs_don();
        item
    }

    fn pdon_distance(&self, item: &DeInterleavingBufferItem) -> u64 {
        if self.pdon < item.decode_order_number as u64 {
            return item.decode_order_number as u64 - self.pdon;
        }
        65535 - self.pdon + item.decode_order_number as u64 + 1
    }

    fn max_abs_don_item(&self) -> Option<&DeInterleavingBufferItem> {
        if self.buffer.is_empty() {
            return None;
        }

        let mut prev_index = 0;
        let mut prev_abs_don = 0;
        for (index, (abs_don, _)) in self.buffer.iter().enumerate() {
            if *abs_don > prev_abs_don {
                prev_index = index;
                prev_abs_don = *abs_don;
            }
        }
        Some(&self.buffer[prev_index].1)
    }

    pub fn try_pop(&mut self) -> Vec<DeInterleavingBufferItem> {
        let mut result = vec![];

        if self.initial_buffering_until.is_some()
            && self.buffer.len() as u64 > self.parameters.sprop_interleaving_depth.unwrap_or(1)
        {
            self.initial_buffering_until = None;
        }

        if let Some(max_diff) = self.parameters.sprop_max_don_diff
            && self.initial_buffering_until.is_some()
        {
            let max_abs_don_item = self.max_abs_don_item().unwrap();
            for (_, item) in &self.buffer {
                if don_diff(
                    item.decode_order_number,
                    max_abs_don_item.decode_order_number,
                ) <= max_diff as i64
                {
                    continue;
                }
                self.initial_buffering_until = None;
                break;
            }
        }

        if let Some(initial_buffering_until) = self.initial_buffering_until {
            if time::Instant::now() < initial_buffering_until {
                return result;
            } else {
                self.initial_buffering_until = None;
            }
        }
        loop {
            if self.buffer.is_empty() {
                break;
            }
            if self.buffer.len() as u64 > self.parameters.sprop_interleaving_depth.unwrap_or(1) {
                result.push(self.try_pop_one().unwrap());
                continue;
            }
            if let Some(max_diff) = self.parameters.sprop_max_don_diff {
                let max_abs_don_item = self.max_abs_don_item().unwrap();
                let mut prev_index = Some(0);
                let mut prev_distance = u64::MAX;
                for (index, (_, item)) in self.buffer.iter().enumerate() {
                    if don_diff(
                        item.decode_order_number,
                        max_abs_don_item.decode_order_number,
                    ) <= max_diff as i64
                    {
                        continue;
                    }
                    let distance = self.pdon_distance(item);
                    if distance < prev_distance {
                        prev_distance = distance;
                        prev_index = Some(index);
                    }
                }

                if let Some(index) = prev_index {
                    result.push(self.buffer.remove(index).unwrap().1);
                    self.calculate_abs_don();
                }
            }
        }
        if let Some(item) = result.last() {
            self.pdon = item.decode_order_number as u64;
        }
        result
    }
}

pub struct RtpH264Sequencer {
    buffer_capacity: usize,
    fragment_buffer_capacity: usize,
    nal_fragment: Option<BytesMut>,
    nal_fragment_don: Option<u16>,
    decode_order_number_cycles: usize,

    packetization_mode: PacketizationMode,

    decoder_buffer: VecDeque<RtpH264BufferItem>,
    de_interleaving_buffer: Option<DeInterleavingBuffer>,
}

impl RtpH264Sequencer {
    pub fn new(
        packetization_mode: PacketizationMode,
        de_interleaving_parameters: DeInterleavingParameters,
    ) -> Self {
        Self {
            buffer_capacity: DEFAULT_BUFFER_CAPACITY,
            fragment_buffer_capacity: DEFAULT_FRAGMENT_BUFFER_CAPACITY,
            nal_fragment: Default::default(),
            nal_fragment_don: Default::default(),
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

    fn enqueue_de_interleaving_buffer(
        &mut self,
        item: DeInterleavingBufferItem,
    ) -> RtpH264Result<()> {
        if let Some(de_interleaving) = &mut self.de_interleaving_buffer {
            de_interleaving.push_back(item);
            de_interleaving.try_pop().into_iter().try_for_each(|item| {
                self.enqueue_decoder_buffer(RtpH264BufferItem {
                    nal_unit: item.nal_unit,
                    rtp_header: item.rtp_header,
                    decode_order_number: Some(item.decode_order_number),
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
                    self.enqueue_de_interleaving_buffer(DeInterleavingBufferItem {
                        nal_unit: item,
                        rtp_header: rtp_header.clone(),
                        decode_order_number: stap_b_packet.decode_order_number,
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

                    self.enqueue_de_interleaving_buffer(DeInterleavingBufferItem {
                        nal_unit: item.nal_unit,
                        rtp_header: rtp_header.clone(),
                        decode_order_number: mtap16_packet
                            .decode_order_number_base
                            .wrapping_add(item.decode_order_number_diff as u16),
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

                    self.enqueue_de_interleaving_buffer(DeInterleavingBufferItem {
                        nal_unit: item.nal_unit,
                        rtp_header: rtp_header.clone(),
                        decode_order_number: mtap24_packet
                            .decode_order_number_base
                            .wrapping_add(item.decode_order_number_diff as u16),
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
        let (fu_header, don, payload) = match packet {
            FragmentedUnit::FuA(packet) => (packet.fu_header, None, packet.payload),
            FragmentedUnit::FuB(packet) => (
                packet.fu_header,
                Some(packet.decode_order_number),
                packet.payload,
            ),
        };

        if let Some(fragmentation_buffer) = &mut self.nal_fragment {
            if fu_header.start_bit {
                tracing::warn!(
                    "got a FU start packet while fragment buffer is not None, dropping previous buffer, length: {}, don: {:?}",
                    fragmentation_buffer.len(),
                    self.nal_fragment_don
                );
                *fragmentation_buffer = BytesMut::from(payload);
                self.nal_fragment_don = don;
            } else {
                if fragmentation_buffer.len() >= self.fragment_buffer_capacity {
                    let dropped_length = fragmentation_buffer.len() + payload.len();
                    let dropped_don = self.nal_fragment_don;

                    self.nal_fragment = None;
                    self.nal_fragment_don = None;
                    return Err(RtpH264Error::SequenceFUPacketsFailed(format!(
                        "fragment buffer exceeds capacity: {}, dropping all data, buffer length: {}, don: {:?}",
                        self.fragment_buffer_capacity, dropped_length, dropped_don
                    )));
                }
                fragmentation_buffer.extend_from_slice(&payload);
            }
        } else if fu_header.start_bit {
            self.nal_fragment = Some(BytesMut::from(payload));
            self.nal_fragment_don = don;
        } else {
            return Err(RtpH264Error::SequenceFUPacketsFailed(
                "got a FU packet without start bit, but fragment buffer is None".to_owned(),
            ));
        }

        if fu_header.end_bit {
            if self.nal_fragment.is_none() {
                return Err(RtpH264Error::SequenceFUPacketsFailed(
                    "got a FU packet with end bit, but fragment buffer is None".to_owned(),
                ));
            } else {
                let reader = Cursor::new(self.nal_fragment.as_ref().unwrap());
                let nalu = NalUnit::read_from(reader)?;

                if let Some(don) = self.nal_fragment_don {
                    self.enqueue_de_interleaving_buffer(DeInterleavingBufferItem {
                        nal_unit: nalu,
                        rtp_header: rtp_header.clone(),
                        decode_order_number: don,
                        timestamp_offset: None,
                    })?;
                } else {
                    self.enqueue_decoder_buffer(RtpH264BufferItem {
                        nal_unit: nalu,
                        rtp_header: rtp_header.clone(),
                        decode_order_number: None,
                        timestamp_offset: None,
                    })?;
                }

                self.nal_fragment = None;
                self.nal_fragment_don = None;
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

    pub fn enqueue(&mut self, packet: RtpH264Packet) -> RtpH264Result<()> {
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

    pub fn try_dump(&mut self) -> Vec<RtpH264BufferItem> {
        let mut result: Vec<RtpH264BufferItem> = Vec::with_capacity(self.decoder_buffer.len());
        while let Some(item) = self.decoder_buffer.pop_front() {
            result.push(item);
        }
        result
    }
}
