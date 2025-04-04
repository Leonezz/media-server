use std::{fmt, str::FromStr};

use sdp_formats::attributes::SDPTrivialAttribute;
use url::Url;

use crate::{errors::RtspMessageError, time::TimeRange};

#[derive(Debug, Clone)]
pub enum RtspSDPControl {
    Absolute(Url),
    Relative(String),
    Asterisk,
}

impl FromStr for RtspSDPControl {
    type Err = RtspMessageError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "*" => Ok(Self::Asterisk),
            s if s.contains("://") => Ok(Self::Absolute(s.parse()?)),
            s => Ok(Self::Relative(s.to_owned())),
        }
    }
}

impl fmt::Display for RtspSDPControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Absolute(url) => write!(f, "control:{}", url),
            Self::Relative(url) => write!(f, "control:{}", url),
            Self::Asterisk => write!(f, "control:*"),
        }
    }
}

impl RtspSDPControl {
    pub fn url_to_str(&self) -> String {
        match self {
            Self::Absolute(url) => url.to_string(),
            Self::Relative(url) => url.clone(),
            Self::Asterisk => "*".to_owned(),
        }
    }
}

impl TryFrom<&SDPTrivialAttribute> for RtspSDPControl {
    type Error = RtspMessageError;
    fn try_from(value: &SDPTrivialAttribute) -> Result<Self, Self::Error> {
        if value.name.ne("control") {
            return Err(RtspMessageError::InvalidSdpControlAttribute(format!(
                "attribute name is not control: {}",
                value.name
            )));
        }
        if value.value.is_none() || value.value.as_ref().unwrap().is_empty() {
            return Err(RtspMessageError::InvalidSdpControlAttribute(
                "the value part of attribute is empty".to_owned(),
            ));
        }

        let result = value.value.as_ref().unwrap().parse().map_err(|err| {
            RtspMessageError::InvalidSdpControlAttribute(format!(
                "parse control attribute failed: {}, {}",
                value.value.as_ref().unwrap(),
                err
            ))
        })?;
        Ok(result)
    }
}

#[derive(Debug)]
pub struct Range(pub TimeRange);

impl FromStr for Range {
    type Err = RtspMessageError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "range:{}", self.0)
    }
}

#[derive(Debug)]
pub struct MTag {
    pub weak: bool,
    pub opaque: String,
}

impl FromStr for MTag {
    type Err = RtspMessageError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(opaque) = s.strip_prefix("W/") {
            return Ok(Self {
                weak: true,
                opaque: opaque.to_owned(),
            });
        }

        Ok(Self {
            weak: false,
            opaque: s.to_owned(),
        })
    }
}
