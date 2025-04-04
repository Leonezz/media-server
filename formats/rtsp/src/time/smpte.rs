use std::{fmt, str::FromStr};

use crate::errors::RtspMessageError;

#[derive(Debug)]
pub struct Smpte {
    pub(crate) hours: u8,
    pub(crate) minutes: u8,
    pub(crate) seconds: u8,
    pub(crate) frames: u8,
    pub(crate) subframes: u8,
}

impl FromStr for Smpte {
    type Err = RtspMessageError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(':');
        let mut parse = |part: &str| {
            parts.next().unwrap_or("0").parse().map_err(|err| {
                RtspMessageError::InvalidSMPTE(format!("parse {} failed: {}, {}", part, s, err))
            })
        };
        Ok(Self {
            hours: parse("hours")?,
            minutes: parse("minutes")?,
            seconds: parse("seconds")?,
            frames: parse("frames")?,
            subframes: parse("subframes")?,
        })
    }
}

impl fmt::Display for Smpte {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}:{}.{}",
            self.hours, self.minutes, self.seconds, self.frames, self.subframes
        )
    }
}
