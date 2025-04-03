use std::fmt;

use crate::errors::RTSPMessageError;

#[derive(Debug, Default, Clone)]
pub enum RtspVersion {
    V1,
    #[default]
    V2,
    Other(String),
}

impl TryFrom<&str> for RtspVersion {
    type Error = RTSPMessageError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "RTSP/1.0" => Ok(Self::V1),
            "RTSP/2.0" => Ok(Self::V2),
            value if value.starts_with("RTSP/1.") || value.starts_with("RTSP/2.") => {
                if value.strip_prefix("RTSP/").unwrap().parse::<f64>().is_err() {
                    return Err(RTSPMessageError::UnknownRtspVersion(Some(value.to_owned())));
                }
                Ok(Self::Other(value.to_owned()))
            }
            _ => Err(RTSPMessageError::UnknownRtspVersion(Some(value.to_owned()))),
        }
    }
}

impl<'a> From<&'a RtspVersion> for &'a str {
    fn from(value: &'a RtspVersion) -> Self {
        match value {
            RtspVersion::V1 => "RTSP/1.0",
            RtspVersion::V2 => "RTSP/2.0",
            RtspVersion::Other(v) => v.as_str(),
        }
    }
}

impl fmt::Display for RtspVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str: &str = self.into();
        f.write_str(str)
    }
}
