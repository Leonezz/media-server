use std::{fmt, str::FromStr};

use sdp_formats::attributes::{SDPTrivialAttribute, extension::SdpAttributeExtension};

#[derive(Debug, Clone)]
pub struct MTag {
    pub weak: bool,
    pub opaque: String,
}

impl FromStr for MTag {
    type Err = sdp_formats::errors::SDPError;
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

impl TryFrom<&sdp_formats::attributes::SDPTrivialAttribute> for MTag {
    type Error = sdp_formats::errors::SDPError;
    fn try_from(value: &sdp_formats::attributes::SDPTrivialAttribute) -> Result<Self, Self::Error> {
        if value.name.ne("mtag") {
            return Err(sdp_formats::errors::SDPError::InvalidAttributeExtension(
                format!("attribute {} not match {} extension", value.name, "mtag"),
            ));
        }
        if value.value.is_none() {
            return Err(sdp_formats::errors::SDPError::InvalidAttributeExtension(
                "no value provided".to_string(),
            ));
        }
        Self::from_str(value.value.as_ref().unwrap())
    }
}

impl fmt::Display for MTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.clone().into_attr().fmt(f)
    }
}

impl SdpAttributeExtension for MTag {
    fn attr_name() -> &'static str {
        "mtag"
    }
    fn value(&self) -> Option<String> {
        Some(if self.weak { "W/" } else { "" }.to_owned() + &self.opaque)
    }
    fn into_value(self) -> Option<String> {
        if self.weak {
            self.value()
        } else {
            Some(self.opaque)
        }
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
