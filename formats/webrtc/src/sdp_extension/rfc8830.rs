//! see: RFC 8830 WebRTC MediaStream Identification in the Session Description Protocol
// msid-value = msid-id [ SP msid-appdata ]
// msid-id = 1*64token-char ; see RFC 4566
// msid-appdata = 1*64token-char ; see RFC 4566

use std::{fmt, str::FromStr};

use sdp_formats::attributes::{SDPTrivialAttribute, extension::SdpAttributeExtension};

#[derive(Debug, Clone)]
pub struct Msid {
    pub id: String,
    pub appdata: Option<String>,
}

impl SdpAttributeExtension for Msid {
    fn attr_name() -> &'static str {
        "msid"
    }
    fn value(&self) -> Option<String> {
        if let Some(appdata) = &self.appdata {
            return Some(format!("{} {}", self.id, appdata));
        }
        Some(self.id.clone())
    }
    fn into_value(self) -> Option<String> {
        if let Some(appdata) = self.appdata {
            return Some([self.id, " ".to_string(), appdata].concat());
        }
        Some(self.id)
    }
    fn into_attr(self) -> sdp_formats::attributes::SDPAttribute {
        sdp_formats::attributes::SDPAttribute::Trivial(
            sdp_formats::attributes::SDPTrivialAttribute {
                name: Self::attr_name().to_owned(),
                value: self.into_value(),
            },
        )
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

impl FromStr for Msid {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (id, data) = s.split_once(' ').unwrap_or((s, ""));
        Ok(Self {
            id: id.to_owned(),
            appdata: if data.is_empty() {
                None
            } else {
                Some(data.to_owned())
            },
        })
    }
}

impl TryFrom<&SDPTrivialAttribute> for Msid {
    type Error = sdp_formats::errors::SDPError;
    fn try_from(value: &SDPTrivialAttribute) -> Result<Self, Self::Error> {
        if value.name != Self::attr_name() {
            return Err(sdp_formats::errors::SDPError::InvalidAttributeExtension(
                format!("attr {} not match extension {}", value, Self::attr_name()),
            ));
        }
        if let Some(v) = &value.value {
            return v
                .parse()
                .map_err(sdp_formats::errors::SDPError::InvalidAttributeExtension);
        }
        Err(sdp_formats::errors::SDPError::InvalidAttributeExtension(
            "no value provided".to_owned(),
        ))
    }
}

impl fmt::Display for Msid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.clone().into_attr().fmt(f)
    }
}
