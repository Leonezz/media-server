use std::{fmt, str::FromStr};

use crate::errors::RtspMessageError;

#[derive(Debug)]
pub struct AbsoluteTimeFormat {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: f64,
}

impl FromStr for AbsoluteTimeFormat {
    type Err = RtspMessageError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() < 16 || !s.ends_with('Z') || s.as_bytes()[8] != b'T' {
            return Err(RtspMessageError::InvalidAbsoluteTime(format!(
                "invalid format: {}",
                s
            )));
        }

        let parse = |start: usize, end: usize, part: &str| {
            s[start..end].parse().map_err(|err| {
                RtspMessageError::InvalidAbsoluteTime(format!(
                    "parse {} failed: {}, {}",
                    part, s, err
                ))
            })
        };

        Ok(Self {
            year: parse(0, 4, "year")?,
            month: parse(4, 6, "month")? as u8,
            day: parse(6, 8, "day")? as u8,
            hour: parse(9, 11, "hour")? as u8,
            minute: parse(11, 13, "minute")? as u8,
            second: s.strip_suffix('Z').unwrap()[13..].parse().map_err(|err| {
                RtspMessageError::InvalidAbsoluteTime(format!(
                    "parse second failed: {}, {}",
                    s, err
                ))
            })?,
        })
    }
}

impl fmt::Display for AbsoluteTimeFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}{}T{}{}{}Z",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}
