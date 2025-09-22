use std::{fmt, str::FromStr};

use sdp_formats::attributes::{SDPTrivialAttribute, extension::SdpAttributeExtension};

use crate::time::TimeRange;

#[derive(Debug, Clone)]
pub struct Range(pub TimeRange);

impl FromStr for Range {
    type Err = sdp_formats::errors::SDPError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse().map_err(|err| {
            sdp_formats::errors::SDPError::InvalidAttributeExtension(format!(
                "parse time range from {} failed: {}",
                s, err
            ))
        })?))
    }
}

impl TryFrom<&SDPTrivialAttribute> for Range {
    type Error = sdp_formats::errors::SDPError;
    fn try_from(value: &SDPTrivialAttribute) -> Result<Self, Self::Error> {
        if value.name.ne("range") {
            return Err(sdp_formats::errors::SDPError::InvalidAttributeExtension(
                format!("attr name {} not match range", value.name),
            ));
        }
        if value.value.is_none() {
            return Err(sdp_formats::errors::SDPError::InvalidAttributeExtension(
                "no value in sdp attr provided".to_owned(),
            ));
        }
        Self::from_str(value.value.as_ref().unwrap())
    }
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.clone().into_attr().fmt(f)
    }
}

impl SdpAttributeExtension for Range {
    fn attr_name() -> &'static str {
        "range"
    }

    fn value(&self) -> Option<String> {
        Some(format!("{}", self.0))
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
                format!("{} not match {} extension", attr, Self::attr_name()),
            )),
        }
    }
}
