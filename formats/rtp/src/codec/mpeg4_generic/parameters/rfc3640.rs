use crate::codec::mpeg4_generic::{
    errors::{RtpMpeg4Error, RtpMpeg4Result},
    parameters::mode::Mode,
};
use bitstream_io::BitRead;
use sdp_formats::attributes::extension::SdpAttributeExtension;
use tokio_util::bytes::Bytes;
use utils::{
    bytes::bytes_to_hex,
    traits::{reader::BitwiseReadFrom, writer::BitwiseWriteTo},
};

#[derive(Debug, Clone)]
pub struct RtpMpeg4Fmtp {
    pub fmt: u8,
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

impl Default for RtpMpeg4Fmtp {
    fn default() -> Self {
        let mut params = Self {
            fmt: 0,
            profile_level_id: 0,
            config: Bytes::new(),
            mode: Mode::AAChbr,
            object_type: None,
            constant_size: None,
            constant_duration: None,
            max_displacement: None,
            de_interleave_buffer_size: None,
            size_length: None,
            index_length: None,
            index_delta_length: None,
            cts_delta_length: None,
            dts_delta_length: None,
            random_access_indication: None,
            stream_state_indication: None,
            auxiliary_data_size_length: None,
        };
        params.reset_default();
        params
    }
}

impl RtpMpeg4Fmtp {
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

impl TryFrom<&RtpMpeg4Fmtp>
    for codec_aac::mpeg4_configuration::audio_specific_config::AudioSpecificConfig
{
    type Error = RtpMpeg4Error;
    fn try_from(value: &RtpMpeg4Fmtp) -> Result<Self, Self::Error> {
        let mut reader = codec_bitstream::reader::BitstreamReader::new(&value.config);
        Ok(
            codec_aac::mpeg4_configuration::audio_specific_config::AudioSpecificConfig::read_from(
                reader.by_ref(),
            )?,
        )
    }
}

impl TryFrom<&codec_aac::mpeg4_configuration::audio_specific_config::AudioSpecificConfig>
    for RtpMpeg4Fmtp
{
    type Error = RtpMpeg4Error;
    fn try_from(
        value: &codec_aac::mpeg4_configuration::audio_specific_config::AudioSpecificConfig,
    ) -> Result<Self, Self::Error> {
        let mut result = Self::default();
        let mut config = vec![];
        let mut writer = bitstream_io::BitWriter::endian(&mut config, bitstream_io::BigEndian);
        value.write_to(&mut writer)?;
        result.config = Bytes::from_owner(bytes_to_hex(&config));
        Ok(result)
    }
}

impl TryFrom<&sdp_formats::attributes::fmtp::FormatParameters> for RtpMpeg4Fmtp {
    type Error = sdp_formats::errors::SDPError;
    fn try_from(
        value: &sdp_formats::attributes::fmtp::FormatParameters,
    ) -> Result<Self, Self::Error> {
        let mut result: Self = value.params.parse().map_err(|err| {
            sdp_formats::errors::SDPError::InvalidAttributeExtension(format!(
                "parse to mpeg4 fmtp failed: {}",
                err
            ))
        })?;
        result.fmt = value.fmt;
        Ok(result)
    }
}

impl From<RtpMpeg4Fmtp> for sdp_formats::attributes::fmtp::FormatParameters {
    fn from(value: RtpMpeg4Fmtp) -> Self {
        Self {
            fmt: value.fmt,
            params: (&value).into(),
        }
    }
}

impl SdpAttributeExtension for RtpMpeg4Fmtp {
    fn attr_name() -> &'static str {
        "fmtp"
    }

    fn into_attr(self) -> sdp_formats::attributes::SDPAttribute {
        sdp_formats::attributes::SDPAttribute::Fmtp(self.into())
    }

    fn try_from_attr(
        attr: &sdp_formats::attributes::SDPAttribute,
    ) -> Result<Self, sdp_formats::errors::SDPError> {
        match attr {
            sdp_formats::attributes::SDPAttribute::Fmtp(fmtp) => fmtp.try_into(),
            _ => Err(sdp_formats::errors::SDPError::InvalidAttributeExtension(
                format!("{} not match mpeg4 fmtp extension", attr),
            )),
        }
    }

    fn value(&self) -> Option<String> {
        Some(format!("{} {}", self.fmt, self))
    }
}
