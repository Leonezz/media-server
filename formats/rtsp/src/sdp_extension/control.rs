use std::{fmt, str::FromStr};

use sdp_formats::attributes::{SDPTrivialAttribute, extension::SdpAttributeExtension};
use url::Url;

use crate::errors::RtspMessageError;

#[derive(Debug, Clone, Default)]
pub enum RtspSDPControl {
    Absolute(Url),
    Relative(String),
    #[default]
    Asterisk,
}

impl RtspSDPControl {
    pub fn new() -> Self {
        Default::default()
    }
}

impl SdpAttributeExtension for RtspSDPControl {
    fn attr_name() -> &'static str {
        "control"
    }
    fn value(&self) -> Option<String> {
        Some(self.url_to_str().to_string())
    }
    fn into_attr(self) -> sdp_formats::attributes::SDPAttribute {
        sdp_formats::attributes::SDPAttribute::Trivial(SDPTrivialAttribute {
            name: Self::attr_name().to_string(),
            value: self.into_value(),
        })
    }
    fn try_from_attr(
        attr: &sdp_formats::attributes::SDPAttribute,
    ) -> Result<Self, sdp_formats::errors::SDPError> {
        match attr {
            sdp_formats::attributes::SDPAttribute::Trivial(trivial) => trivial.try_into(),
            _ => Err(sdp_formats::errors::SDPError::InvalidAttributeExtension(
                format!("attr {} not match extension {}", attr, Self::attr_name()),
            )),
        }
    }
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

impl RtspSDPControl {
    pub fn url_to_str(&self) -> &str {
        match self {
            Self::Absolute(url) => url.as_str(),
            Self::Relative(url) => url,
            Self::Asterisk => "*",
        }
    }
}

impl TryFrom<&SDPTrivialAttribute> for RtspSDPControl {
    type Error = sdp_formats::errors::SDPError;
    fn try_from(value: &SDPTrivialAttribute) -> Result<Self, Self::Error> {
        if value.name.ne("control") {
            return Err(sdp_formats::errors::SDPError::InvalidAttributeExtension(
                format!("attribute name is not control: {}", value.name),
            ));
        }
        if value.value.is_none() || value.value.as_ref().unwrap().is_empty() {
            return Err(sdp_formats::errors::SDPError::InvalidAttributeExtension(
                "the value part of attribute is empty".to_owned(),
            ));
        }

        let result = value.value.as_ref().unwrap().parse().map_err(|err| {
            sdp_formats::errors::SDPError::InvalidAttributeExtension(format!(
                "parse control attribute failed: {}, {}",
                value.value.as_ref().unwrap(),
                err
            ))
        })?;
        Ok(result)
    }
}

impl fmt::Display for RtspSDPControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.clone().into_attr().fmt(f)
    }
}
