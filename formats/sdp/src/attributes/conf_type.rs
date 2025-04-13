use std::{fmt, str::FromStr};

use crate::errors::SDPError;

#[derive(Debug)]
pub enum ConferenceType {
    Broadcast,
    Meeting,
    Moderated,
    Test,
    H332,
}

impl FromStr for ConferenceType {
    type Err = SDPError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "broadcast" => Ok(Self::Broadcast),
            "meeting" => Ok(Self::Meeting),
            "moderated" => Ok(Self::Moderated),
            "test" => Ok(Self::Test),
            "H332" => Ok(Self::H332),
            _ => Err(SDPError::InvalidAttributeLine(format!(
                "unknown conference type: {}",
                s
            ))),
        }
    }
}

impl fmt::Display for ConferenceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Broadcast => f.write_str("broadcast"),
            Self::Meeting => f.write_str("meeting"),
            Self::Moderated => f.write_str("moderated"),
            Self::Test => f.write_str("test"),
            Self::H332 => f.write_str("H332"),
        }
    }
}
