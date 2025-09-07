use crate::codec::h264::{errors::RtpH264Error, packet::sequencer::RtpH264BufferItem};
use utils::traits::buffer::GenericFragmentComposer;

// we must respect the frame boundaries, i.e., nalus at the same timestamp if of the same frame
#[derive(Default)]
pub struct TimestampGrouper {
    buffer: Option<RtpH264BufferItem>,
}

impl TimestampGrouper {
    pub fn new() -> Self {
        Default::default()
    }
}

impl GenericFragmentComposer for TimestampGrouper {
    type Error = RtpH264Error;
    type In = RtpH264BufferItem;
    type Out = RtpH264BufferItem;
    fn enqueue(&mut self, packet: Self::In) -> Result<Option<Self::Out>, Self::Error> {
        if self.buffer.is_none() {
            self.buffer = Some(packet);
            return Ok(None);
        }
        let buffer = self.buffer.as_mut().unwrap();
        if packet.rtp_header.timestamp == buffer.rtp_header.timestamp {
            buffer.merge(packet);
        } else if packet.rtp_header.timestamp > buffer.rtp_header.timestamp {
            let out = self.buffer.replace(packet);
            return Ok(out);
        } else {
            panic!(
                "input packet rtp_header: {:?}, buffer rtp_header: {:?}",
                packet.rtp_header, buffer.rtp_header
            );
        }
        Ok(None)
    }
}
