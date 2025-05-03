use std::{fmt, str::FromStr};

use tokio_util::bytes::Bytes;

use super::errors::{RtpMpeg4Error, RtpMpeg4Result};

pub mod read;
pub mod stream_type;
pub mod write;
#[derive(Debug, Default, Clone, Copy)]
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

#[derive(Debug, Default, Clone)]
pub struct RtpMpeg4OutOfBandParams {
    pub profile_level_id: u16,
    pub config: Bytes,
    pub mode: Mode,
    pub object_type: Option<u8>,
    pub constant_size: Option<u64>, // The sizeLength and the constantSize parameters MUST NOT be simultaneously present.
    pub constant_duration: Option<u64>,
    pub max_displacement: Option<u64>,
    pub de_interleave_buffer_size: Option<u64>,
    pub size_length: Option<u64>, // The sizeLength and the constantSize parameters MUST NOT be simultaneously present.
    pub index_length: Option<u64>,
    pub index_delta_length: Option<u64>,
    pub cts_delta_length: Option<u64>,
    pub dts_delta_length: Option<u64>,
    pub random_access_indication: Option<bool>, // default to false
    pub stream_state_indication: Option<u64>, // SHALL NOT be present for MPEG-4 audio and MPEG-4 video streams
    pub auxiliary_data_size_length: Option<u64>,
}

impl RtpMpeg4OutOfBandParams {
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode
    }
    pub fn is_single_au_mode(&self) -> bool {
        self.size_length.unwrap_or(self.constant_size.unwrap_or(0)) == 0
    }
    pub fn reset_default(&mut self) {
        self.size_length = self.default_au_size_length();
        self.index_length = self.default_au_index_size_length();
        self.index_delta_length = self.default_au_index_size_length();
    }
    pub fn default_au_size_length(&self) -> Option<u64> {
        match self.mode {
            Mode::Generic => None,
            Mode::CELPcbr => None,
            Mode::CELPvbr => Some(6),
            Mode::AAClbr => Some(6),
            Mode::AAChbr => Some(13),
        }
    }
    pub fn default_au_index_size_length(&self) -> Option<u64> {
        match self.mode {
            Mode::Generic => None,
            Mode::CELPcbr => None,
            Mode::CELPvbr => Some(2),
            Mode::AAClbr => Some(2),
            Mode::AAChbr => Some(3),
        }
    }
    pub fn allow_fragmentation(&self) -> bool {
        match self.mode {
            Mode::CELPcbr => false,
            Mode::CELPvbr => false,
            Mode::AAClbr => false,
            Mode::AAChbr => true,
            Mode::Generic => true,
        }
    }
    pub fn allow_interleaving(&self) -> bool {
        match self.mode {
            Mode::CELPcbr => false,
            Mode::CELPvbr => true,
            Mode::AAClbr => true,
            Mode::AAChbr => true,
            Mode::Generic => true,
        }
    }
    pub fn allow_auxiliary(&self) -> bool {
        match self.mode {
            Mode::CELPcbr => false,
            Mode::CELPvbr => false,
            Mode::AAClbr => false,
            Mode::AAChbr => false,
            Mode::Generic => true,
        }
    }

    pub fn allow_au_headers(&self) -> bool {
        match self.mode {
            Mode::CELPcbr => false,
            Mode::CELPvbr => true,
            Mode::AAClbr => true,
            Mode::AAChbr => true,
            Mode::Generic => true,
        }
    }

    pub fn must_has_au_headers(&self) -> bool {
        if !self.allow_au_headers() {
            return false;
        }

        match self.mode {
            Mode::CELPvbr => return true,
            Mode::AAClbr => return true,
            Mode::AAChbr => return true,
            _ => {}
        }
        false
    }

    pub fn guess_has_au_headers(&self) -> bool {
        if self.size_length.is_none()
            && self.index_length.is_none()
            && self.index_delta_length.is_none()
            && self.cts_delta_length.is_none()
            && self.dts_delta_length.is_none()
            && self.stream_state_indication.is_none()
            && self.random_access_indication.is_none()
        {
            return false;
        }
        true
    }

    pub fn validate(&self) -> RtpMpeg4Result<()> {
        // The constantSize and the sizeLength parameters MUST NOT be simultaneously present.
        if self.constant_size.is_some() && self.size_length.is_some() {
            return Err(RtpMpeg4Error::SyntaxError(
                "got both constantSize and sizeLength present".to_owned(),
            ));
        }

        if !self.allow_au_headers()
            && (self.size_length.unwrap_or(0) != 0
                || self.cts_delta_length.unwrap_or(0) != 0
                || self.dts_delta_length.unwrap_or(0) != 0
                || self.index_delta_length.unwrap_or(0) != 0
                || self.index_length.unwrap_or(0) != 0
                || self.random_access_indication.unwrap_or(false)
                || self.stream_state_indication.unwrap_or(0) != 0)
        {
            return Err(RtpMpeg4Error::SyntaxError(format!(
                "got au-header releated params {} while in {} mode",
                self, self.mode,
            )));
        }

        if !self.allow_auxiliary() && (self.auxiliary_data_size_length.unwrap_or(0) != 0) {
            return Err(RtpMpeg4Error::SyntaxError(format!(
                "got auxiliary related params {} while in {} mode",
                self, self.mode
            )));
        }

        if self.must_has_au_headers()
            && (self.size_length.unwrap_or(0) == 0
                || self.index_length.unwrap_or(0) == 0
                || self.index_delta_length.unwrap_or(0) == 0)
        {
            return Err(RtpMpeg4Error::SyntaxError(format!(
                "no au-header related params {} provided while in {} mode",
                self, self.mode
            )));
        }

        if let Mode::CELPcbr = self.mode
            && self.constant_size.is_none()
        {
            return Err(RtpMpeg4Error::SyntaxError(
                "no constantSize provided while in CELP-cbr mode".to_owned(),
            ));
        }
        Ok(())
    }
}
