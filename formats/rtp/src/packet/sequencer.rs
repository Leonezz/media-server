use std::{cmp, collections::VecDeque};

use utils::random;

use crate::{
    codec::{
        h264::packet::sequencer::RtpH264BufferItem,
        mpeg4_generic::packet::sequencer::RtpMpeg4GenericBufferItem,
    },
    errors::RtpError,
    sequence_number::SequenceNumber,
};

use super::RtpTrivialPacket;

#[derive(Debug)]
pub enum RtpBufferVideoItem {
    H264(RtpH264BufferItem),
}

#[derive(Debug)]
pub enum RtpBufferAudioItem {
    AAC(RtpMpeg4GenericBufferItem),
}

#[derive(Debug)]
pub enum RtpBufferItem {
    Video(RtpBufferVideoItem),
    Audio(RtpBufferAudioItem),
}

pub trait RtpBufferedSequencer {
    fn enqueue(&mut self, packet: RtpTrivialPacket) -> Result<(), RtpError>;
    fn try_dump(&mut self) -> Vec<RtpBufferItem>;
}

pub trait GenericSequencer {
    type In;
    type Out;
    type Error;
    fn enqueue(&mut self, packet: Self::In) -> Result<(), Self::Error>;
    fn try_dump(&mut self) -> Vec<Self::Out>;
}

pub trait GenericFragmentComposer {
    type In;
    type Out;
    type Error;
    fn enqueue(&mut self, packet: Self::In) -> Result<Option<Self::Out>, Self::Error>;
}

/// RtpTrivialSequencer takes rtp packets from outside systems,
/// and one can obtain RtpTrivialPacket from it with sequence number being continuous
pub struct RtpTrivialSequencer {
    capacity: usize,
    initial_buffer_packets: usize,
    initial_buffering: bool,
    next_sequence_number: SequenceNumber,
    buffer: VecDeque<RtpTrivialPacket>,
}

impl RtpTrivialSequencer {
    pub fn new(capacity: usize, initial_buffer_packets: usize) -> Self {
        Self {
            capacity,
            initial_buffer_packets,
            initial_buffering: true,
            next_sequence_number: SequenceNumber::new(0, 0),
            buffer: VecDeque::with_capacity(capacity),
        }
    }

    pub fn timestamp_minmax(&self) -> Option<(u32, u32)> {
        if self.buffer.is_empty() {
            return None;
        }

        Some(
            self.buffer
                .iter()
                .fold((u32::MAX, u32::MIN), |(min, max), item| {
                    (
                        cmp::min(min, item.header.timestamp),
                        cmp::max(max, item.header.timestamp),
                    )
                }),
        )
    }

    pub fn smallest_sequence_number_item_index(&self) -> Option<(u16, usize)> {
        if self.buffer.is_empty() {
            return None;
        }
        let mut result = (u16::MAX, 0);
        for (i, item) in self.buffer.iter().enumerate() {
            if item.header.sequence_number < result.0 {
                result = (item.header.sequence_number, i);
            }
        }
        Some(result)
    }
}

impl GenericSequencer for RtpTrivialSequencer {
    type In = RtpTrivialPacket;
    type Out = RtpTrivialPacket;
    type Error = RtpError;
    fn enqueue(&mut self, packet: Self::In) -> Result<(), Self::Error> {
        self.buffer.push_back(packet);
        Ok(())
    }

    fn try_dump(&mut self) -> Vec<Self::Out> {
        let _span =
            tracing::debug_span!("rtp sequencer dump", queue_size = self.buffer.len()).entered();
        if self.buffer.is_empty() {
            return vec![];
        }
        if self.initial_buffering && self.buffer.len() < self.initial_buffer_packets {
            return vec![];
        }

        let mut result = vec![];
        if self.initial_buffering {
            self.initial_buffering = false;
            let (min_seq, index) = self.smallest_sequence_number_item_index().unwrap();
            self.next_sequence_number.set_round(0);
            self.next_sequence_number.set_number(min_seq);
            self.next_sequence_number.add_number(1);
            result.push(self.buffer.remove(index).unwrap());
        }
        while let Some((min_seq, index)) = self.smallest_sequence_number_item_index() {
            if self.next_sequence_number.number() < min_seq && self.buffer.len() < self.capacity / 4
            {
                if random::random_u64() % 1000 == 0 {
                    tracing::debug!(
                        "interleaved rtp packets detected, waiting. expected seq: {}, min seq: {}",
                        self.next_sequence_number.number(),
                        min_seq
                    );
                }
                break;
            }
            if self.next_sequence_number.number() > min_seq {
                let item = self.buffer.remove(index).unwrap();
                if self.next_sequence_number.number() - min_seq < 10000 {
                    tracing::warn!("outdated rtp packets detected: {:?}", item.header);
                } else {
                    tracing::trace!(
                        "rtp sequence number might wrapped, adjuest next_sequence_number: {}",
                        min_seq
                    );
                    result.push(item);
                    self.next_sequence_number.set_number(min_seq);
                    self.next_sequence_number.add_number(1);
                }
                continue;
            }
            // here: min_seq = self.next_seqence_number
            result.push(self.buffer.remove(index).unwrap());
            self.next_sequence_number.set_number(min_seq);
            self.next_sequence_number.add_number(1);
        }

        while self.buffer.len() > self.capacity
            && let Some((min_seq, index)) = self.smallest_sequence_number_item_index()
        {
            tracing::warn!(
                "sequencer buffer overflow, dump smallest sequence number item: {}",
                min_seq
            );
            result.push(self.buffer.remove(index).unwrap());
            self.next_sequence_number.set_number(min_seq);
            self.next_sequence_number.add_number(1);
        }
        result
    }
}
