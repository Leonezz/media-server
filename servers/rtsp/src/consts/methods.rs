//! @see: RFC 7826 Table 7

use crate::errors::RTSPMessageError;

pub mod method_names {
    pub const DESCRIBE: &str = "DESCRIBE";
    pub const GET_PARAMETER: &str = "GET_PARAMETER";
    pub const OPTIONS: &str = "OPTIONS";
    pub const PAUSE: &str = "PAUSE";
    pub const PLAY: &str = "PLAY";
    pub const PLAY_NOTIFY: &str = "PLAY_NOTIFY";
    pub const REDIRECT: &str = "REDIRECT";
    pub const SETUP: &str = "SETUP";
    pub const SET_PARAMETER: &str = "SET_PARAMETER";
    pub const TEARDOWN: &str = "TEARDOWN";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtspMethod {
    Describe,
    GetParameter,
    Options,
    Pause,
    Play,
    PlayNotify,
    Redirect,
    Setup,
    SetParameter,
    TearDown,
}

impl From<RtspMethod> for &'static str {
    fn from(value: RtspMethod) -> Self {
        match value {
            RtspMethod::Describe => method_names::DESCRIBE,
            RtspMethod::GetParameter => method_names::GET_PARAMETER,
            RtspMethod::Options => method_names::OPTIONS,
            RtspMethod::Pause => method_names::PAUSE,
            RtspMethod::Play => method_names::PLAY,
            RtspMethod::PlayNotify => method_names::PLAY_NOTIFY,
            RtspMethod::Redirect => method_names::REDIRECT,
            RtspMethod::Setup => method_names::SETUP,
            RtspMethod::SetParameter => method_names::SET_PARAMETER,
            RtspMethod::TearDown => method_names::TEARDOWN,
        }
    }
}

impl TryFrom<&str> for RtspMethod {
    type Error = RTSPMessageError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            method_names::DESCRIBE => Ok(Self::Describe),
            method_names::GET_PARAMETER => Ok(Self::GetParameter),
            method_names::OPTIONS => Ok(Self::Options),
            method_names::PAUSE => Ok(Self::Pause),
            method_names::PLAY => Ok(Self::PlayNotify),
            method_names::PLAY_NOTIFY => Ok(Self::PlayNotify),
            method_names::REDIRECT => Ok(Self::Redirect),
            method_names::SETUP => Ok(Self::Setup),
            method_names::SET_PARAMETER => Ok(Self::SetParameter),
            method_names::TEARDOWN => Ok(Self::TearDown),
            _ => Err(RTSPMessageError::UnknownMethod(value.into())),
        }
    }
}
