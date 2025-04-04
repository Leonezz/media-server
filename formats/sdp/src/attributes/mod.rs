pub mod conf_type;
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
        match self {
            #[allow(deprecated)]
            Self::Cat(cat) => write!(f, "cat:{}", cat),
            #[allow(deprecated)]
            Self::Keywds(keywds) => write!(f, "keywds:{}", keywds),
            Self::Tool(tool) => write!(f, "tool:{}", tool),
            Self::PTime(ptime) => write!(f, "ptime:{}", ptime),
            Self::MaxPTime(max_ptime) => write!(f, "maxptime:{}", max_ptime),
            Self::RtpMap(rtpmap) => write!(f, "rtpmap:{}", rtpmap),
            Self::MediaDirection(direction) => write!(f, "{}", direction),
            Self::Orient(orient) => write!(f, "orient:{}", orient),
            Self::Type(tp) => write!(f, "type:{}", tp),
            Self::Charset(cs) => write!(f, "charset:{}", cs),
            Self::SDPLang(lang) => write!(f, "sdplang:{}", lang),
            Self::Lang(lang) => write!(f, "lang:{}", lang),
            Self::Framerate(fr) => write!(f, "framerate:{}", fr),
            Self::Quality(qu) => write!(f, "quality:{}", qu),
            Self::Fmtp(fmtp) => write!(f, "fmtp:{}", fmtp),
            Self::Trivial(trivial) => {
                write!(f, "{}", trivial.name)?;
                if let Some(value) = &trivial.value {
                    write!(f, ":{}", value)?;
                }
                Ok(())
            }
        }
    }
}
