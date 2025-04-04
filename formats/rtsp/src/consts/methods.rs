//! @see: RFC 7826 Table 7

use std::{fmt, str::FromStr};

use crate::errors::RtspMessageError;

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
    pub const RECORD: &str = "RECORD";
    // v1.0
    pub const ANNOUNCE: &str = "ANNOUNCE";
}

pub const RTSP_METHODS: [&str; 12] = [
    method_names::DESCRIBE,
    method_names::GET_PARAMETER,
    method_names::OPTIONS,
    method_names::PAUSE,
    method_names::PLAY,
    method_names::PLAY_NOTIFY,
    method_names::REDIRECT,
    method_names::SETUP,
    method_names::SET_PARAMETER,
    method_names::TEARDOWN,
    method_names::ANNOUNCE,
    method_names::RECORD,
];

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
    Announce,
    Record,
}

impl From<&RtspMethod> for &'static str {
    fn from(value: &RtspMethod) -> Self {
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
            RtspMethod::Announce => method_names::ANNOUNCE,
            RtspMethod::Record => method_names::RECORD,
        }
    }
}

impl FromStr for RtspMethod {
    type Err = RtspMessageError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            method_names::DESCRIBE => Ok(Self::Describe),
            method_names::GET_PARAMETER => Ok(Self::GetParameter),
            method_names::OPTIONS => Ok(Self::Options),
            method_names::PAUSE => Ok(Self::Pause),
            method_names::PLAY => Ok(Self::Play),
            method_names::PLAY_NOTIFY => Ok(Self::PlayNotify),
            method_names::REDIRECT => Ok(Self::Redirect),
            method_names::SETUP => Ok(Self::Setup),
            method_names::SET_PARAMETER => Ok(Self::SetParameter),
            method_names::TEARDOWN => Ok(Self::TearDown),
            method_names::ANNOUNCE => Ok(Self::Announce),
            method_names::RECORD => Ok(Self::Record),
            _ => Err(RtspMessageError::UnknownMethod(Some(s.into()))),
        }
    }
}

impl fmt::Display for RtspMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str: &str = self.into();
        f.write_str(str)
    }
}
