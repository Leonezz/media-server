use std::{fmt, str::FromStr};

use crate::errors::SDPError;

#[derive(Debug)]
pub struct FormatParameters {
    fmt: u8,
    params: String,
}

impl FromStr for FormatParameters {
    type Err = SDPError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (first, second) = s
            .split_once(' ')
            .ok_or_else(|| SDPError::InvalidAttributeLine(format!("invalid fmtp: {}", s)))?;
        let fmt: u8 = first.parse().map_err(|err| {
            SDPError::InvalidAttributeLine(format!("parse fmtp fmt failed: {}, {}", first, err))
        })?;

        Ok(Self {
            fmt,
            params: second.to_owned(),
        })
    }
}

impl fmt::Display for FormatParameters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.fmt, self.params)
    }
}
