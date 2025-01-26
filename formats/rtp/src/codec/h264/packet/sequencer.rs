use std::{collections::VecDeque, io::Cursor};

use h264_codec::nalu::NalUnit;
use tokio_util::bytes::BytesMut;
use utils::traits::reader::ReadFrom;

use crate::{
    codec::h264::{
        RtpH264NalUnit, aggregation::AggregationNalUnits, fragmented::FragmentedUnit,
        single_nalu::SingleNalUnit,
    },
    errors::{RtpError, RtpResult},
    header::RtpHeader,
};

use super::RtpH264Packet;

#[derive(Debug)]
pub struct RtpH264BufferItem {
    pub nal_unit: NalUnit,
    pub rtp_header: RtpHeader,
    pub decode_order_number: Option<u16>,
    pub timestamp_offset: Option<u32>,
}

const DEFAULT_BUFFER_CAPACITY: usize = 1000;
const DEFAULT_FRAGMENT_BUFFER_CAPACITY: usize = 50000;

pub struct RtpH264Sequencer {
    buffer_capacity: usize,
    fragment_buffer_capacity: usize,
    nal_unit_buffer: VecDeque<RtpH264BufferItem>,
    nal_fragment: Option<BytesMut>,
    nal_fragment_don: Option<u16>,
    decode_order_number_cycles: usize,
}

impl Default for RtpH264Sequencer {
    fn default() -> Self {
        Self {
            buffer_capacity: DEFAULT_BUFFER_CAPACITY,
            fragment_buffer_capacity: DEFAULT_FRAGMENT_BUFFER_CAPACITY,
            nal_unit_buffer: VecDeque::with_capacity(DEFAULT_BUFFER_CAPACITY),
            nal_fragment: Default::default(),
            nal_fragment_don: Default::default(),
            decode_order_number_cycles: 0,
        }
    }
}

impl RtpH264Sequencer {
    fn enqueue_buffer_item(&mut self, item: RtpH264BufferItem) -> RtpResult<()> {
        if self.nal_unit_buffer.len() >= self.buffer_capacity {
            let dropped = self.nal_unit_buffer.pop_front();
            tracing::warn!("dropped item from rtp h264 sequencer: {:?}", dropped);
        }

        self.nal_unit_buffer.push_back(item);
        Ok(())
    }

    fn on_aggregated_packet(
        &mut self,
        rtp_header: RtpHeader,
        packet: AggregationNalUnits,
    ) -> RtpResult<()> {
        match packet {
            AggregationNalUnits::StapA(stap_a_packet) => {
                stap_a_packet.nal_units.into_iter().try_for_each(|item| {
                    self.enqueue_buffer_item(RtpH264BufferItem {
                        nal_unit: item,
                        rtp_header: rtp_header.clone(),
                        decode_order_number: None,
                        timestamp_offset: None,
                    })
                })?;
            }
            AggregationNalUnits::StapB(stap_b_packet) => {
                stap_b_packet.nal_units.into_iter().try_for_each(|item| {
                    self.enqueue_buffer_item(RtpH264BufferItem {
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

                    self.enqueue_buffer_item(RtpH264BufferItem {
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

                    self.enqueue_buffer_item(RtpH264BufferItem {
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

    fn on_fragmented(&mut self, rtp_header: RtpHeader, packet: FragmentedUnit) -> RtpResult<()> {
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
                    return Err(RtpError::SequenceFUPacketsFailed(format!(
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
            return Err(RtpError::SequenceFUPacketsFailed(
                "got a FU packet without start bit, but fragment buffer is None".to_owned(),
            ));
        }

        if fu_header.end_bit {
            if self.nal_fragment.is_none() {
                return Err(RtpError::SequenceFUPacketsFailed(
                    "got a FU packet with end bit, but fragment buffer is None".to_owned(),
                ));
            } else {
                let reader = Cursor::new(self.nal_fragment.as_ref().unwrap());
                let nalu = NalUnit::read_from(reader)?;

                self.enqueue_buffer_item(RtpH264BufferItem {
                    nal_unit: nalu,
                    rtp_header: rtp_header.clone(),
                    decode_order_number: self.nal_fragment_don,
                    timestamp_offset: None,
                })?;

                self.nal_fragment = None;
                self.nal_fragment_don = None;
            }
        }

        Ok(())
    }

    pub fn enqueue(&mut self, packet: RtpH264Packet) -> RtpResult<()> {
        let rtp_header = packet.header;
        match packet.payload {
            RtpH264NalUnit::SingleNalu(SingleNalUnit(nalu)) => {
                self.enqueue_buffer_item(RtpH264BufferItem {
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
        let mut result: Vec<RtpH264BufferItem> = Vec::with_capacity(self.nal_unit_buffer.len());
        while let Some(item) = self.nal_unit_buffer.pop_front() {
            result.push(item);
        }
        result
    }
}
