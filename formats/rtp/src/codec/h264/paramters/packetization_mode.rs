use std::{fmt, str::FromStr};

use crate::codec::h264::errors::RtpH264Error;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PacketizationMode {
    SingleNalu,
    NonInterleaved,
    Interleaved,
}

impl FromStr for PacketizationMode {
    type Err = RtpH264Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "0" => Ok(Self::SingleNalu),
            "1" => Ok(Self::NonInterleaved),
            "2" => Ok(Self::Interleaved),
            _ => Err(RtpH264Error::InvalidPacketizationMode(format!(
                "unknown packetization mode: {}",
                s
            ))),
        }
    }
}

impl fmt::Display for PacketizationMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SingleNalu => write!(f, "0"),
            Self::NonInterleaved => write!(f, "1"),
            Self::Interleaved => write!(f, "2"),
        }
    }
}
