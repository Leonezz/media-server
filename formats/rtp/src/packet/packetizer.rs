use codec_h264::nalu::NalUnit;
use tokio_util::bytes::{Bytes, BytesMut};

use crate::{errors::RtpError, header::RtpHeader};

use super::RtpTrivialPacket;

#[derive(Debug, Default)]
pub struct RtpTrivialPacketBuilder {
    header: RtpHeader,
    payload: BytesMut,
}

impl RtpTrivialPacketBuilder {
    pub fn header(mut self, header: RtpHeader) -> Self {
        self.header = header;
        self
    }
    pub fn version(mut self, version: u8) -> Self {
        self.header.version = version;
        self
    }

    pub fn payload(mut self, payload: &[u8]) -> Self {
        self.payload.extend_from_slice(payload);
        self
    }

    pub fn build(self) -> RtpTrivialPacket {
        RtpTrivialPacket::new(self.header, self.payload.freeze())
    }
}

#[derive(Debug)]
pub struct RtpTrivialPacketizerH264Item {
    pub nalus: Vec<NalUnit>,
}

#[derive(Debug)]
pub struct RtpTrivialPacketizerAACItem {
    pub access_units: Vec<Bytes>,
}

#[derive(Debug)]
pub enum RtpPacketizerVideoItem {
    H264(RtpTrivialPacketizerH264Item),
}

#[derive(Debug)]
pub enum RtpPacketizerAudioItem {
    AAC(RtpTrivialPacketizerAACItem),
}

#[derive(Debug)]
pub enum RtpPacketizerItem {
    Video(RtpPacketizerVideoItem),
    Audio(RtpPacketizerAudioItem),
}

pub trait RtpTrivialPacketPacketizer {
    fn set_rtp_header(&mut self, header: RtpHeader);
    fn set_frame_timestamp(&mut self, timestamp: u64);
    fn get_rtp_clockrate(&self) -> u64;
    fn rtp_header(&self) -> &RtpHeader;
    fn packetize(&mut self, item: RtpPacketizerItem) -> Result<(), RtpError>;
    fn build(&mut self) -> Result<Vec<RtpTrivialPacket>, RtpError>;
}

pub fn wallclock_to_rtp_timestamp(
    ts_ms: u64,
    base_wallclock_ms: u64,
    base_rtp_ts: u64,
    clockrate: u64,
) -> u64 {
    let delta_ms = ts_ms.saturating_sub(base_wallclock_ms);
    let delta_rtp = (delta_ms * clockrate) / 1000;
    base_rtp_ts.wrapping_add(delta_rtp)
}
