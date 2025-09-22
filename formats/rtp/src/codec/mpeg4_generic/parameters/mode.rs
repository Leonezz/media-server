use crate::codec::mpeg4_generic::errors::RtpMpeg4Error;
use std::{fmt, str::FromStr};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Generic,
    CELPcbr,
    CELPvbr,
    AAClbr,
    AAChbr,
}

impl FromStr for Mode {
    type Err = RtpMpeg4Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "generic" => Ok(Self::Generic),
            "celp-cbr" => Ok(Self::CELPcbr),
            "celp-vbr" => Ok(Self::CELPvbr),
            "aac-lbr" => Ok(Self::AAClbr),
            "aac-hbr" => Ok(Self::AAChbr),
            _ => Err(RtpMpeg4Error::InvalidMode(s.to_owned())),
        }
    }
}

impl From<&Mode> for &str {
    fn from(value: &Mode) -> Self {
        match value {
            Mode::Generic => "generic",
            Mode::CELPcbr => "CELP-cbr",
            Mode::CELPvbr => "CELP-vbr",
            Mode::AAClbr => "AAC-lbr",
            Mode::AAChbr => "AAC-hbr",
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s: &str = self.into();
        write!(f, "{}", s)
    }
}
