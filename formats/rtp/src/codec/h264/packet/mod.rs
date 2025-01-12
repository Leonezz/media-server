use crate::header::RtpHeader;

use super::H264RtpNalUnit;

#[derive(Debug)]
pub struct RtpH264Packet {
    pub header: RtpHeader,
    pub payload: H264RtpNalUnit,
}
