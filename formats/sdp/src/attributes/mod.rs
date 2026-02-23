pub mod conf_type;
pub mod extension;
pub mod fmtp;
pub mod media_direction;
pub mod orient;
pub mod rtpmap;
use std::{fmt, str::FromStr};

use conf_type::ConferenceType;
use fmtp::FormatParameters;
use media_direction::MediaDirection;
use orient::Orient;
use rtpmap::RtpMap;

use crate::{CRLF, errors::SDPError};

/// 5.13. Attributes ("a=")
/// a=<attribute-name>
/// a=<attribute-name>:<attribute-value>
#[derive(Debug, Clone)]
pub struct SDPTrivialAttribute {
    pub name: String,
    pub value: Option<String>,
}

impl fmt::Display for SDPTrivialAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a={}", self.name)?;
        if let Some(value) = &self.value {
            write!(f, ":{}{}", value, CRLF)?;
        } else {
            write!(f, "{}", CRLF)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum SDPAttribute {
    Trivial(SDPTrivialAttribute),
    #[deprecated]
    Cat(String),
    #[deprecated]
    Keywds(String),
    Tool(String),
    PTime(f64),    // TODO: bound to none zero
    MaxPTime(f64), // TODO: bound to none zero
    RtpMap(RtpMap),
    MediaDirection(MediaDirection),
    Orient(Orient),
    Type(ConferenceType),
    Charset(String), // TODO: bound with current chaset list
    SDPLang(String), // TODO: bound with language list
    Lang(String),    // TODO: bound with language list
    Framerate(f64),  // TODO: bound to none zero
    Quality(u64),    // TODO: bound to none zero
    Fmtp(FormatParameters),
}

impl SDPAttribute {
    pub fn name(&self) -> &str {
        match self {
            #[allow(deprecated)]
            Self::Cat(_) => "cat",
            #[allow(deprecated)]
            Self::Keywds(_) => "keywds",
            Self::Tool(_) => "tool",
            Self::PTime(_) => "ptime",
            Self::MaxPTime(_) => "maxptime",
            Self::RtpMap(_) => "rtpmap",
            Self::MediaDirection(d) => d.to_str(),
            Self::Orient(_) => "orient",
            Self::Type(_) => "type",
            Self::Charset(_) => "charset",
            Self::SDPLang(_) => "sdplang",
            Self::Lang(_) => "lang",
            Self::Framerate(_) => "framerate",
            Self::Quality(_) => "quality",
            Self::Fmtp(_) => "fmtp",
            Self::Trivial(trivial) => trivial.name.as_ref(),
        }
    }
}

impl FromStr for SDPAttribute {
    type Err = SDPError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (k, v) = s.split_once(':').unwrap_or((s, ""));
        match k {
            #[allow(deprecated)]
            "cat" => Ok(Self::Cat(v.to_owned())),
            #[allow(deprecated)]
            "keywds" => Ok(Self::Keywds(v.to_owned())),
            "tool" => Ok(Self::Tool(v.to_owned())),
            "ptime" => Ok(Self::PTime(v.parse().map_err(|err| {
                SDPError::InvalidAttributeLine(format!("parse ptime failed: {}, {}", v, err))
            })?)),
            "maxptime" => Ok(Self::MaxPTime(v.parse().map_err(|err| {
                SDPError::InvalidAttributeLine(format!("parse maxptime failed: {}, {}", v, err))
            })?)),
            "rtpmap" => Ok(Self::RtpMap(v.parse()?)),
            "recvonly" | "sendrecv" | "sendonly" | "inactive" => {
                Ok(Self::MediaDirection(k.parse().unwrap()))
            }
            "orient" => Ok(Self::Orient(v.parse()?)),
            "type" => Ok(Self::Type(v.parse()?)),
            "charset" => Ok(Self::Charset(v.to_owned())),
            "sdplang" => Ok(Self::SDPLang(v.to_owned())),
            "lang" => Ok(Self::Lang(v.to_owned())),
            "framerate" => Ok(Self::Framerate(v.parse().map_err(|err| {
                SDPError::InvalidAttributeLine(format!("parse framerate failed: {}, {}", v, err))
            })?)),
            "quality" => Ok(Self::Quality(v.parse().map_err(|err| {
                SDPError::InvalidAttributeLine(format!("parse quality failed: {}, {}", v, err))
            })?)),
            "fmtp" => Ok(Self::Fmtp(v.parse()?)),
            _ => Ok(Self::Trivial(SDPTrivialAttribute {
                name: k.to_owned(),
                value: if v.is_empty() {
                    None
                } else {
                    Some(v.to_owned())
                },
            })),
        }
    }
}

impl fmt::Display for SDPAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())?;
        match self {
            #[allow(deprecated)]
            Self::Cat(cat) => write!(f, ":{}", cat),
            #[allow(deprecated)]
            Self::Keywds(keywds) => write!(f, ":{}", keywds),
            Self::Tool(tool) => write!(f, ":{}", tool),
            Self::PTime(ptime) => write!(f, ":{}", ptime),
            Self::MaxPTime(max_ptime) => write!(f, ":{}", max_ptime),
            Self::RtpMap(rtpmap) => write!(f, ":{}", rtpmap),
            Self::MediaDirection(_) => Ok(()),
            Self::Orient(orient) => write!(f, ":{}", orient),
            Self::Type(tp) => write!(f, ":{}", tp),
            Self::Charset(cs) => write!(f, ":{}", cs),
            Self::SDPLang(lang) => write!(f, ":{}", lang),
            Self::Lang(lang) => write!(f, ":{}", lang),
            Self::Framerate(fr) => write!(f, ":{}", fr),
            Self::Quality(qu) => write!(f, ":{}", qu),
            Self::Fmtp(fmtp) => write!(f, ":{}", fmtp),
            Self::Trivial(trivial) => {
                if let Some(value) = &trivial.value {
                    write!(f, ":{}", value)?;
                }
                Ok(())
            }
        }
    }
}
