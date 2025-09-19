use crate::errors::SDPError;
use std::{fmt, str::FromStr};

#[derive(Debug, Clone)]
pub struct RtpMap {
    pub payload_type: u8,
    pub encoding_name: String,
    pub clock_rate: u64,
    pub encoding_params: Option<u64>,
}

impl FromStr for RtpMap {
    type Err = SDPError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (first, second) = s
            .split_once(' ')
            .ok_or_else(|| SDPError::InvalidAttributeLine(format!("invalid rtpmap: {}", s)))?;
        let payload_type: u8 = first.parse().map_err(|err| {
            SDPError::InvalidAttributeLine(format!(
                "parse rtpmap payload type failed: {}, {}",
                first, err
            ))
        })?;
        let fields: Vec<_> = second.split('/').collect();
        if fields.len() < 2 || fields.len() > 3 {
            return Err(SDPError::InvalidAttributeLine(format!(
                "rtpmap line is invalid: {}",
                second
            )));
        }

        let encoding_name = fields[0].to_owned();
        let clock_rate: u64 = fields[1].parse().map_err(|err| {
            SDPError::InvalidAttributeLine(format!(
                "parse rtpmap clock rate failed: {}, {}",
                fields[1], err
            ))
        })?;
        Ok(Self {
            payload_type,
            encoding_name,
            clock_rate,
            encoding_params: if fields.len() == 3 {
                Some(fields[2].parse().map_err(|err| {
                    SDPError::InvalidAttributeLine(format!(
                        "parse rtpmap encoding params failed: {}, {}",
                        fields[2], err
                    ))
                })?)
            } else {
                None
            },
        })
    }
}

impl fmt::Display for RtpMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {}/{}",
            self.payload_type, self.encoding_name, self.clock_rate
        )?;
        if let Some(param) = self.encoding_params {
            write!(f, "/{}", param)?;
        }
        Ok(())
    }
}
