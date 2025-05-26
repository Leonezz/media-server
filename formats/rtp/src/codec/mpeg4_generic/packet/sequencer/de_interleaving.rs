use std::collections::VecDeque;

use utils::traits::buffer::GenericSequencer;

use crate::codec::mpeg4_generic::errors::RtpMpeg4Error;

use super::RtpMpeg4GenericBufferItem;

pub struct RtpMpeg4GenericDeInterleavingBuffer {
    de_interleaving_buffer_capacity: usize,
    max_displacement: u64,
    buffer: VecDeque<RtpMpeg4GenericBufferItem>,

    next_au_index: u64,
    initial_buffer_size: usize,
    initial_buffering: bool,
}

impl RtpMpeg4GenericDeInterleavingBuffer {
    pub fn new(capacity: usize, max_displacement: u64, initial_buffer_size: usize) -> Self {
        Self {
            de_interleaving_buffer_capacity: capacity,
            max_displacement,
            buffer: VecDeque::with_capacity(capacity),
            next_au_index: 0,
            initial_buffer_size,
            initial_buffering: true,
        }
    }
    fn smallest_au_index_item_index(&self) -> Option<(u64, usize)> {
        if self.buffer.is_empty() {
            return None;
        }
        let mut result = (u64::MAX, 0);
        for (i, item) in self.buffer.iter().enumerate() {
            let au_index = item
                .access_unit
                .header
                .au_index
                .expect("no au index is set for item of deinterleaving buffer")
                + item.rtp_header.sequence_number as u64;
            if au_index < result.0 {
                result = (au_index, i);
            }
        }
        Some(result)
    }

    fn max_timestamp_diff(&self) -> Option<(u32, usize)> {
        if self.buffer.len() < 2 {
            return None;
        }
        let mut result = (u32::MAX, 0, u32::MIN, 0);
        for (i, item) in self.buffer.iter().enumerate() {
            if item.access_unit.timestamp < result.0 {
                result.0 = item.access_unit.timestamp;
                result.1 = i;
            }
            if item.access_unit.timestamp > result.2 {
                result.2 = item.access_unit.timestamp;
                result.3 = i;
            }
        }
        Some((result.2 - result.0, result.1))
    }
}

impl GenericSequencer for RtpMpeg4GenericDeInterleavingBuffer {
    type Error = RtpMpeg4Error;
    type In = Vec<RtpMpeg4GenericBufferItem>;
    type Out = RtpMpeg4GenericBufferItem;
    fn enqueue(&mut self, mut packet: Self::In) -> Result<(), Self::Error> {
        let _ = packet.iter_mut().fold(None, |prev, item| {
            item.access_unit.header.au_index = Some(if let Some(prev) = prev {
                prev + item.access_unit.header.au_index_delta.unwrap_or(0) + 1
            } else {
                item.access_unit.header.au_index.unwrap_or(0)
            });
            Some(item.access_unit.header.au_index.unwrap())
        });

        self.buffer.extend(packet);
        Ok(())
    }

    fn try_dump(&mut self) -> Vec<Self::Out> {
        if self.buffer.is_empty() {
            return vec![];
        }
        if self.initial_buffering && self.buffer.len() < self.initial_buffer_size {
            return vec![];
        }
        let mut result = vec![];
        if self.initial_buffering {
            self.initial_buffering = false;
            let (_, index) = self.smallest_au_index_item_index().unwrap_or((0, 0));

            let item = self.buffer.remove(index).unwrap();
            self.next_au_index = item.access_unit.header.au_index.unwrap_or(0)
                + item.rtp_header.sequence_number as u64
                + 1;
            result.push(item);
        }
        while let Some((min_au_index, index)) = self.smallest_au_index_item_index() {
            if self.next_au_index < min_au_index {
                tracing::debug!(
                    "interleaved rtp packets detected, waiting. expected au_index: {}, min au_index: {}",
                    self.next_au_index,
                    min_au_index
                );
                break;
            }
            if self.next_au_index > min_au_index {
                let item = self.buffer.remove(index).unwrap();
                if self.next_au_index - min_au_index < 10000 {
                    tracing::warn!(
                        "outdated packets detected: expected au_index: {}, min au_index: {}, rtp_header: {:?}, au_header: {:?}",
                        self.next_au_index,
                        min_au_index,
                        item.rtp_header,
                        item.access_unit.header
                    );
                } else {
                    tracing::info!(
                        "au_index might be wrapped, adjuest next_au_index: {}",
                        min_au_index
                    );
                    result.push(item);
                    self.next_au_index = min_au_index + 1;
                }
                continue;
            }
            // here: min_au_index = self.next_au_index
            result.push(self.buffer.remove(index).unwrap());
            self.next_au_index = min_au_index + 1;
        }

        while self.buffer.len() > self.de_interleaving_buffer_capacity
            && let Some((max_ts_diff, min_ts_index)) = self.max_timestamp_diff()
            && max_ts_diff as u64 > self.max_displacement
        {
            let dropped_item = self.buffer.remove(min_ts_index).unwrap();
            tracing::warn!(
                "de_interleaving_buffer overflow max_displacement, drop earliest one. rtp_header: {:?}, au_header: {:?}",
                dropped_item.rtp_header,
                dropped_item.access_unit.header
            );
        }

        result
    }
}
