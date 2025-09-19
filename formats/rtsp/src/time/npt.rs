use std::{fmt, str::FromStr};

use crate::errors::RtspMessageError;

#[derive(Debug)]
pub enum Npt {
    Seconds(f64),
    HHMMSS {
        hours: u64,
        minutes: u8,
        seconds: f64,
    },
    Now,
}

impl FromStr for Npt {
    type Err = RtspMessageError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "now" => Ok(Self::Now),
            s if s.contains(':') => {
                let parts: Vec<_> = s.split(':').collect();
                if parts.len() != 3 {
                    return Err(RtspMessageError::InvalidNPT(format!(
                        "parse npt hhmmss mode failed: {}",
                        s
                    )));
                }

                Ok(Self::HHMMSS {
                    hours: parts[0].parse().map_err(|err| {
                        RtspMessageError::InvalidNPT(format!(
                            "parse npt hhmmss mode hours failed: {}, {}",
                            parts[0], err
                        ))
                    })?,
                    minutes: parts[1].parse().map_err(|err| {
                        RtspMessageError::InvalidNPT(format!(
                            "parse npt hhmmss mode minutes failed: {}, {}",
                            parts[1], err
                        ))
                    })?,
                    seconds: parts[2].parse().map_err(|err| {
                        RtspMessageError::InvalidNPT(format!(
                            "parse npt hhmmss mode seconds failed: {}, {}",
                            parts[2], err
                        ))
                    })?,
                })
            }
            s => Ok(Self::Seconds(s.parse().map_err(|err| {
                RtspMessageError::InvalidNPT(format!("parse npt in seconds mode: {}, {}", s, err))
            })?)),
        }
    }
}

impl fmt::Display for Npt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Now => write!(f, "now"),
            Self::Seconds(v) => write!(f, "{}", v),
            Self::HHMMSS {
                hours,
                minutes,
                seconds,
            } => write!(f, "{}:{}:{}", hours, minutes, seconds),
        }
    }
}
