use std::{fmt, str::FromStr};

use absolute::AbsoluteTimeFormat;
use npt::Npt;
use smpte::Smpte;

use crate::errors::RtspMessageError;

pub mod absolute;
pub mod npt;
pub mod smpte;

#[derive(Debug)]
pub enum MediaTimeFormat {
    SMPTE {
        tc: String,
        framerate: f64,
        value: Smpte,
    },
    NPT(Npt),
    Absolute(AbsoluteTimeFormat),
    Extension(String),
}

impl fmt::Display for MediaTimeFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Absolute(abs) => write!(f, "{}", abs),
            Self::Extension(ext) => write!(f, "{}", ext),
            Self::NPT(npt) => write!(f, "{}", npt),
            Self::SMPTE {
                tc: _,
                framerate: _,
                value,
            } => write!(f, "{}", value),
        }
    }
}

#[derive(Debug)]
pub struct TimeRange {
    pub start_time: Option<MediaTimeFormat>,
    pub end_time: Option<MediaTimeFormat>,
}

impl FromStr for TimeRange {
    type Err = RtspMessageError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (first, second) = s.split_once('=').unwrap_or((s, ""));
        let (start, end) = second.split_once('-').ok_or_else(|| {
            RtspMessageError::InvalidSdpRangeAttribute(format!(
                "invalid npt time format: {}",
                second
            ))
        })?;
        match first {
            "npt" => Ok(Self {
                start_time: if start.is_empty() {
                    None
                } else {
                    Some(MediaTimeFormat::NPT(start.parse()?))
                },
                end_time: if end.is_empty() {
                    None
                } else {
                    Some(MediaTimeFormat::NPT(end.parse()?))
                },
            }),
            "clock" => Ok(Self {
                start_time: if start.is_empty() {
                    None
                } else {
                    Some(MediaTimeFormat::Absolute(start.parse()?))
                },
                end_time: if end.is_empty() {
                    None
                } else {
                    Some(MediaTimeFormat::Absolute(end.parse()?))
                },
            }),
            other if other.starts_with("smpte") => {
                let tc = other.split_once('-').unwrap_or(("", "")).1;
                let framerate: f64 = match tc {
                    "" => 30.0,
                    "30" => 30.0,
                    "30-drop" => 29.97,
                    "25" => 25.0,
                    _ => 30.0,
                };
                Ok(Self {
                    start_time: if start.is_empty() {
                        None
                    } else {
                        Some(MediaTimeFormat::SMPTE {
                            tc: tc.to_owned(),
                            framerate,
                            value: start.parse()?,
                        })
                    },
                    end_time: if end.is_empty() {
                        None
                    } else {
                        Some(MediaTimeFormat::SMPTE {
                            tc: tc.to_owned(),
                            framerate,
                            value: end.parse()?,
                        })
                    },
                })
            }
            _ => Ok(Self {
                start_time: if start.is_empty() {
                    None
                } else {
                    Some(MediaTimeFormat::Extension(start.to_owned()))
                },
                end_time: if end.is_empty() {
                    None
                } else {
                    Some(MediaTimeFormat::Extension(end.to_owned()))
                },
            }),
        }
    }
}

impl fmt::Display for TimeRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.start_time.as_ref().or(self.end_time.as_ref()) {
            None => {}
            Some(MediaTimeFormat::SMPTE {
                tc,
                framerate: _,
                value: _,
            }) => {
                if tc.is_empty() {
                    write!(f, "smpte=")?;
                } else {
                    write!(f, "smpte-{}=", tc)?;
                }
            }
            Some(MediaTimeFormat::Absolute(_)) => {
                write!(f, "clock=")?;
            }
            Some(MediaTimeFormat::NPT(_)) => {
                write!(f, "npt=")?;
            }
            Some(MediaTimeFormat::Extension(_)) => {}
        }
        if let Some(start) = &self.start_time {
            write!(f, "{}", start)?;
        }
        write!(f, "-")?;
        if let Some(end) = &self.end_time {
            write!(f, "{}", end)?;
        }
        Ok(())
    }
}
