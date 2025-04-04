use std::{fmt, str::FromStr};

use crate::errors::SDPError;

#[derive(Debug, Clone, Copy)]
pub enum Orient {
    Portrait,
    Landscape,
    Seascape,
}

impl FromStr for Orient {
    type Err = SDPError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "portrait" => Ok(Self::Portrait),
            "landscape" => Ok(Self::Landscape),
            "seascape" => Ok(Self::Seascape),
            _ => Err(SDPError::InvalidAttributeLine(format!(
                "unknown orient: {}",
                s
            ))),
        }
    }
}

impl fmt::Display for Orient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Portrait => f.write_str("portrait"),
            Self::Landscape => f.write_str("landscape"),
            Self::Seascape => f.write_str("seascape"),
        }
    }
}
