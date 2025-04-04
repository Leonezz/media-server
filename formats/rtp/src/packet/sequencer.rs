use crate::{
    codec::h264::packet::sequencer::RtpH264BufferItem,
    errors::{RtpError, RtpResult},
};

use super::RtpTrivialPacket;

#[derive(Debug)]
pub enum RtpBufferVideoItem {
    H264(RtpH264BufferItem),
}

#[derive(Debug)]
pub enum RtpBufferItem {
    Video(RtpBufferVideoItem),
    Audio(),
}

pub trait RtpBufferedSequencer {
    fn enqueue(&mut self, packet: RtpTrivialPacket) -> Result<(), RtpError>;
    fn try_dump(&mut self) -> Vec<RtpBufferItem>;
}
