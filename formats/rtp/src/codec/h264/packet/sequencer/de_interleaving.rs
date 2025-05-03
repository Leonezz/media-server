use std::{collections::VecDeque, time};

use crate::{
    codec::h264::{paramters::H264SDPFormatParameters, util::don_diff},
    errors::RtpError,
    packet::sequencer::GenericSequencer,
};

use super::{DEFAULT_BUFFER_CAPACITY, RtpH264BufferItem};

#[derive(Debug, Default)]
pub struct RtpH264DeInterleavingParameters {
    pub sprop_interleaving_depth: Option<u64>,
    pub sprop_deint_buf_req: Option<u64>,
    pub sprop_init_buf_time: Option<u64>,
    pub sprop_max_don_diff: Option<u64>,
}

impl From<H264SDPFormatParameters> for RtpH264DeInterleavingParameters {
    fn from(value: H264SDPFormatParameters) -> Self {
        Self {
            sprop_interleaving_depth: value.sprop_interleaving_depth.map(|v| v.into()),
            sprop_deint_buf_req: value.sprop_deint_buf_req,
            sprop_init_buf_time: value.sprop_init_buf_time,
            sprop_max_don_diff: value.sprop_max_don_diff.map(|v| v.into()),
        }
    }
}

// TODO: make deinterleaving buffer also a sequencer
#[derive(Debug)]
pub struct DeInterleavingBuffer {
    parameters: RtpH264DeInterleavingParameters,

    initial_buffering_until: Option<time::Instant>,
    pdon: u64,
    buffer: VecDeque<(u64, RtpH264BufferItem)>,
}
impl DeInterleavingBuffer {
    pub fn new(parameters: RtpH264DeInterleavingParameters) -> Self {
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
        let mut prev_don = self.buffer[0].1.decode_order_number.unwrap() as u64;
        let mut prev_abs_don = prev_don;
        for (abs_don, item) in &mut self.buffer {
            let current_don = item.decode_order_number.unwrap() as u64;
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

    fn try_pop_one(&mut self) -> Option<RtpH264BufferItem> {
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

    fn pdon_distance(&self, item: &RtpH264BufferItem) -> u64 {
        if self.pdon < item.decode_order_number.unwrap() as u64 {
            return item.decode_order_number.unwrap() as u64 - self.pdon;
        }
        65535 - self.pdon + item.decode_order_number.unwrap() as u64 + 1
    }

    fn max_abs_don_item(&self) -> Option<&RtpH264BufferItem> {
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
}

impl GenericSequencer for DeInterleavingBuffer {
    type In = RtpH264BufferItem;
    type Out = RtpH264BufferItem;
    type Error = RtpError;
    fn enqueue(&mut self, packet: Self::In) -> Result<(), Self::Error> {
        self.buffer.push_back((0, packet));
        self.calculate_abs_don();
        Ok(())
    }

    fn try_dump(&mut self) -> Vec<Self::Out> {
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
                    item.decode_order_number.unwrap(),
                    max_abs_don_item.decode_order_number.unwrap(),
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
                        item.decode_order_number.unwrap(),
                        max_abs_don_item.decode_order_number.unwrap(),
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
            self.pdon = item.decode_order_number.unwrap() as u64;
        }
        result
    }
}
