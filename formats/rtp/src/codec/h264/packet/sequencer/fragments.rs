use std::{
    collections::HashMap,
    io::{self, Read},
};

use codec_h264::nalu::NalUnit;
use tokio_util::bytes::{BufMut, BytesMut};
use utils::traits::{buffer::GenericFragmentComposer, reader::ReadFrom};

use crate::{
    codec::h264::{errors::RtpH264Error, fragmented::FragmentedUnit},
    header::RtpHeader,
};

use super::RtpH264BufferItem;

pub struct FragmentItem {
    fragment: BytesMut,
    don: Option<u16>,
}
pub struct RtpH264FragmentsBuffer {
    nal_fragments: HashMap<u32, FragmentItem>,
    fragment_buffer_capacity: usize,
}

impl RtpH264FragmentsBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            nal_fragments: HashMap::new(),
            fragment_buffer_capacity: capacity,
        }
    }
}

impl GenericFragmentComposer for RtpH264FragmentsBuffer {
    type In = (RtpHeader, FragmentedUnit);
    type Out = RtpH264BufferItem;
    type Error = RtpH264Error;
    fn enqueue(
        &mut self,
        (rtp_header, packet): Self::In,
    ) -> Result<Option<Self::Out>, Self::Error> {
        let (indicator, fu_header, don, payload) = match packet {
            FragmentedUnit::FuA(packet) => {
                (packet.indicator, packet.fu_header, None, packet.payload)
            }
            FragmentedUnit::FuB(packet) => (
                packet.indicator,
                packet.fu_header,
                Some(packet.decode_order_number),
                packet.payload,
            ),
        };
        if fu_header.start_bit {
            // first fragment
            if let Some(fragmentation_buffer) = self.nal_fragments.get_mut(&rtp_header.timestamp) {
                // already has a fragment with the same timestamp, might be a duplicate
                tracing::warn!(
                    "got a FU start packet while fragment buffer is not None, dropping previous buffer, length: {}, don: {:?}, fu_header: {:?}",
                    fragmentation_buffer.fragment.len(),
                    fragmentation_buffer.don,
                    fu_header,
                );
                fragmentation_buffer.fragment = BytesMut::new();
                fragmentation_buffer
                    .fragment
                    .put_u8((indicator.nal_ref_idc << 5) | (fu_header.nalu_type)); // F and NRI from indicator, and NaluType from fu_header
                fragmentation_buffer.fragment.extend_from_slice(&payload);
                fragmentation_buffer.don = don;
            } else {
                // happy path, insert new fragment item
                let mut buffer = BytesMut::new();
                buffer.put_u8((indicator.nal_ref_idc << 5) | (fu_header.nalu_type)); // F and NRI from indicator, and NaluType from fu_header
                buffer.extend_from_slice(&payload);
                self.nal_fragments.insert(
                    rtp_header.timestamp,
                    FragmentItem {
                        fragment: buffer,
                        don,
                    },
                );
            }
        } else if let Some(fragmentation_buffer) = self.nal_fragments.get_mut(&rtp_header.timestamp)
        {
            // happy path, not first fragment, and already have fragment with the same timestamp
            if fragmentation_buffer.fragment.len() >= self.fragment_buffer_capacity
                && !fu_header.end_bit
            {
                // not going to end, but exceeds the fragment buffer
                let dropped_length = fragmentation_buffer.fragment.len() + payload.len();
                let dropped_don = fragmentation_buffer.don;
                self.nal_fragments.remove(&rtp_header.timestamp);
                return Err(RtpH264Error::SequenceFUPacketsFailed(format!(
                    "fragment buffer exceeds capacity: {}, dropping all data, buffer length: {}, don: {:?}, fu_header: {:?}",
                    self.fragment_buffer_capacity, dropped_length, dropped_don, fu_header
                )));
            } else {
                // happy path, just extend fragmentation
                fragmentation_buffer.fragment.extend_from_slice(&payload);
                if don.is_some() {
                    fragmentation_buffer.don = don;
                }
            }
        } else {
            // not the first fragment coming, but the first one might be missing
            return Err(RtpH264Error::SequenceFUPacketsFailed(format!(
                "got a FU packet without start bit, but fragment buffer is None, rtp_header: {:?}, fu_header: {:?}",
                rtp_header, fu_header
            )));
        }

        if fu_header.end_bit {
            // fragment of this timestamp should be complete
            if let Some(fragmentation_buffer) = self.nal_fragments.remove(&rtp_header.timestamp) {
                // happy path, gather fragmentation and try to parse as nalu
                let mut reader = io::Cursor::new(fragmentation_buffer.fragment.as_ref());
                let nalu = NalUnit::read_from(reader.by_ref())?;
                let don = fragmentation_buffer.don;
                return Ok(Some(RtpH264BufferItem {
                    nal_unit: nalu,
                    rtp_header: rtp_header.clone(),
                    decode_order_number: don,
                    timestamp_offset: None,
                }));
            } else {
                // end-bit, but no previously cached fragments
                return Err(RtpH264Error::SequenceFUPacketsFailed(format!(
                    "got a FU packet with end bit, but fragment buffer is None, rtp_header: {:?}, fu_header: {:?}",
                    rtp_header, fu_header
                )));
            }
        }
        Ok(None)
    }
}
