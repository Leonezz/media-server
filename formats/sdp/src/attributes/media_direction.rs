use std::{fmt, str::FromStr};

use crate::errors::SDPError;

#[derive(Debug, Default, Clone, Copy)]
pub enum MediaDirection {
    #[default]
    SendRecv,
    RecvOnly,
    SendOnly,
    Inactive,
}

impl MediaDirection {
    pub fn to_str(&self) -> &str {
        match self {
            Self::SendRecv => "sendrecv",
            Self::RecvOnly => "recvonly",
            Self::SendOnly => "sendonly",
            Self::Inactive => "inactive",
        }
    }
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
        f.write_str(self.to_str())
    }
}
