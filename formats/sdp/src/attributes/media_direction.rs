use std::{fmt, str::FromStr};

use crate::errors::SDPError;

#[derive(Debug, Default)]
pub enum MediaDirection {
    #[default]
    SendRecv,
    RecvOnly,
    SendOnly,
    Inactive,
}

impl FromStr for MediaDirection {
    type Err = SDPError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sendrecv" => Ok(Self::SendRecv),
            "recvonly" => Ok(Self::RecvOnly),
            "sendonly" => Ok(Self::SendOnly),
            "inactive" => Ok(Self::Inactive),
            _ => Err(SDPError::InvalidAttributeLine(format!(
                "unknown media direction: {}",
                s,
            ))),
        }
    }
}

impl fmt::Display for MediaDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SendRecv => f.write_str("sendrecv"),
            Self::RecvOnly => f.write_str("recvonly"),
            Self::SendOnly => f.write_str("sendonly"),
            Self::Inactive => f.write_str("inactive"),
        }
    }
}
