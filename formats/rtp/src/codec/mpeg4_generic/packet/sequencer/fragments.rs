use std::collections::HashMap;

use utils::traits::buffer::GenericFragmentComposer;

use crate::{
    codec::mpeg4_generic::{access_unit::AccessUnitFragment, errors::RtpMpeg4Error},
    header::RtpHeader,
};

use super::RtpMpeg4GenericBufferItem;

pub struct RtpMpeg4GenericFragmentBufferItem {
    fragment: AccessUnitFragment,
}

pub struct RtpMpeg4GenericFragmentationBuffer {
    buffer: HashMap<u32, RtpMpeg4GenericFragmentBufferItem>,
    fragment_buffer_capacity: usize,
}

impl RtpMpeg4GenericFragmentationBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: HashMap::new(),
            fragment_buffer_capacity: capacity,
        }
    }
}

impl GenericFragmentComposer for RtpMpeg4GenericFragmentationBuffer {
    type Error = RtpMpeg4Error;
    type In = (RtpHeader, AccessUnitFragment);
    type Out = RtpMpeg4GenericBufferItem;
    fn enqueue(&mut self, packet: Self::In) -> Result<Option<Self::Out>, Self::Error> {
        let (rtp_header, packet) = packet;
        if let Some(fragment) = self.buffer.get_mut(&rtp_header.timestamp) {
            if fragment.fragment.body.len() + packet.body.len() > self.fragment_buffer_capacity {
                let item = self.buffer.remove(&rtp_header.timestamp).unwrap();
                tracing::warn!(
                    "fragment buffer overflow: {}, drop item: {:?}",
                    self.fragment_buffer_capacity,
                    item.fragment.header
                );
            } else {
                fragment.fragment.body.extend_from_slice(&packet.body);
            }
        } else {
            self.buffer.insert(
                rtp_header.timestamp,
                RtpMpeg4GenericFragmentBufferItem { fragment: packet },
            );
        }

        if rtp_header.marker {
            return Ok(Some(RtpMpeg4GenericBufferItem {
                access_unit: self
                    .buffer
                    .remove(&rtp_header.timestamp)
                    .unwrap()
                    .fragment
                    .complete(),
                rtp_header,
            }));
        }

        Ok(None)
    }
}
