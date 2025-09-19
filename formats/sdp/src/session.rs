//! @see: RFC 8866 SDP: Session Description Protocol
use crate::{
    CRLF,
    attributes::{SDPAttribute, fmtp::FormatParameters, rtpmap::RtpMap},
    errors::SDPError,
    reader::SessionDescriptionReader,
};
use std::{fmt, io, str::FromStr};
use url::Url;
use utils::traits::reader::ReadFrom;

/// 5.1. Protocol Version ("v=")
/// v=0
pub type SDPVersion = u32;

/// 5.2. Origin ("o=")
/// o=<username> <sess-id> <sess-version> <nettype> <addrtype> <unicast-address>
#[derive(Debug, Default, Clone)]
pub enum SDPNetType {
    #[default]
    IN,
    Other(String),
}

impl From<SDPNetType> for String {
    fn from(value: SDPNetType) -> Self {
        format!("{}", value)
    }
}

impl From<&str> for SDPNetType {
    fn from(value: &str) -> Self {
        match value {
            "IN" => Self::IN,
            other => Self::Other(other.into()),
        }
    }
}

impl fmt::Display for SDPNetType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::IN => "IN",
                Self::Other(str) => str,
            }
        )
    }
}

#[derive(Debug, Default, Clone)]
pub enum SDPAddrType {
    #[default]
    IP4,
    IP6,
    Other(String),
}

impl From<SDPAddrType> for String {
    fn from(value: SDPAddrType) -> Self {
        format!("{}", value)
    }
}

impl From<&str> for SDPAddrType {
    fn from(value: &str) -> Self {
        match value {
            "IP4" => Self::IP4,
            "IP6" => Self::IP6,
            other => Self::Other(other.into()),
        }
    }
}

impl fmt::Display for SDPAddrType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::IP4 => "IP4",
                Self::IP6 => "IP6",
                Self::Other(str) => str,
            }
        )
    }
}

#[derive(Debug, Default, Clone)]
pub struct SDPOrigin {
    pub user_name: String,
    pub session_id: u64,
    pub session_version: u64,
    pub net_type: SDPNetType,
    pub addr_type: SDPAddrType,
    pub unicast_address: String,
}

impl fmt::Display for SDPOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "o={} {} {} {} {} {}{}",
            self.user_name,
            self.session_id,
            self.session_version,
            self.net_type,
            self.addr_type,
            self.unicast_address,
            CRLF
        )
    }
}

/// 5.3. Session Name ("s=")
/// s=<session name>
pub type SDPSessionName = String;

/// 5.4. Session Information ("i=")
/// i=<session information>
pub type SDPSessionInformation = String;

/// 5.5. URI ("u=")
/// u=<uri>
pub type SDPUri = Url;

/// 5.6. Email Address and Phone Number ("e=" and "p=")
/// e=<email-address>
/// p=<phone-number>
pub type SDPEmail = String;
pub type SDPPhoneNumber = String;

/// 5.7. Connection Information ("c=")
/// c=<nettype> <addrtype> <connection-address>
#[derive(Debug, Default, Clone)]
pub struct SDPAddress {
    pub address: String,
    pub ttl: Option<u64>,
    pub range: Option<u64>,
}

impl fmt::Display for SDPAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.address)?;
        if let Some(ttl) = self.ttl {
            write!(f, "/{}", ttl)?;
        }
        if let Some(range) = self.range {
            write!(f, "/{}", range)?;
        }
        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct SDPConnectionInformation {
    pub net_type: SDPNetType,
    pub addr_type: SDPAddrType,
    pub connection_address: SDPAddress,
}

impl fmt::Display for SDPConnectionInformation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "c={} {} {}{}",
            self.net_type, self.addr_type, self.connection_address, CRLF
        )
    }
}

#[derive(Debug, Clone)]
pub enum SDPBandwidthType {
    AS, // AS
    CT, // CT
    Other(String),
}

impl FromStr for SDPBandwidthType {
    type Err = SDPError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "AS" => Ok(Self::AS),
            "CT" => Ok(Self::CT),
            _ => Ok(Self::Other(s.to_owned())),
        }
    }
}

impl fmt::Display for SDPBandwidthType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::AS => "AS",
                Self::CT => "CT",
                Self::Other(str) => str,
            }
        )
    }
}

/// 5.8. Bandwidth Information ("b=")
/// b=<bwtype>:<bandwidth>
#[derive(Debug, Clone)]
pub struct SDPBandWidthInformation {
    pub bw_type: SDPBandwidthType,
    pub bandwidth: u64,
}

impl fmt::Display for SDPBandWidthInformation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "b={}:{}{}", self.bw_type, self.bandwidth, CRLF)
    }
}

/// 5.9. Time Active ("t=")
/// t=<start-time> <stop-time>
/// 5.10. Repeat Times ("r=")
/// r=<repeat interval> <active duration> <offsets from start-time>
#[derive(Debug, Clone)]
pub struct SDPRepeatTime {
    // in seconds
    pub interval: i64,
    // in seconds
    pub duration: i64,
    // in seconds
    pub offsets: Vec<i64>,
    pub time_zone_adjustment: Vec<SDPTimeZoneAdjustment>,
}

impl fmt::Display for SDPRepeatTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "r={} {}", self.interval, self.duration)?;
        self.offsets
            .iter()
            .try_for_each(|item| write!(f, " {}", item))?;
        write!(f, "{}", CRLF)?;
        if !self.time_zone_adjustment.is_empty() {
            write!(f, "z={}", self.time_zone_adjustment[0])?;
            self.time_zone_adjustment[1..]
                .iter()
                .try_for_each(|item| write!(f, " {}", item))?;
            write!(f, "{}", CRLF)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SDPTimeInformation {
    pub start_time: u64,
    pub stop_time: u64,
    pub repeat_times: Vec<SDPRepeatTime>,
}

impl fmt::Display for SDPTimeInformation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "t={} {}", self.start_time, self.stop_time)?;
        write!(f, "{}", CRLF)?;
        if !self.repeat_times.is_empty() {
            self.repeat_times
                .iter()
                .try_for_each(|item| write!(f, "{}", item))?;
        }
        Ok(())
    }
}

/// 5.11. Time Zone Adjustment ("z=")
/// z=<adjustment time> <offset> <adjustment time> <offset> ....
#[derive(Debug, Clone)]
pub struct SDPTimeZoneAdjustment {
    pub adjustment_time: i64,
    // in seconds
    pub offset: i64,
}

impl fmt::Display for SDPTimeZoneAdjustment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.adjustment_time, self.offset)
    }
}

/// 5.12. Encryption Keys ("k=")
/// k=<method>
/// k=<method>:<encryption key>
/// obsolete
#[derive(Debug, Clone)]
pub struct SDPEncryptionKeys {
    pub method: String,
    pub key: Option<String>,
}

impl fmt::Display for SDPEncryptionKeys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "k={}", self.method)?;
        if let Some(key) = &self.key {
            write!(f, ":{}{}", key, CRLF)?;
        } else {
            write!(f, "{}", CRLF)?;
        }
        Ok(())
    }
}

/// 5.14. Media Descriptions ("m=")
/// m=<media> <port> <proto> <fmt> ...
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub enum SDPMediaType {
    #[default]
    Audio, // audio
    Video,       // video
    Text,        // text
    Application, // application
    Message,     // message
    Image,       // image
    Other(String),
}

impl fmt::Display for SDPMediaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Audio => "audio",
                Self::Video => "video",
                Self::Text => "text",
                Self::Application => "application",
                Self::Message => "message",
                Self::Image => "image",
                Self::Other(str) => str,
            }
        )
    }
}

impl From<&str> for SDPMediaType {
    fn from(value: &str) -> Self {
        match value {
            "audio" => Self::Audio,
            "video" => Self::Video,
            "text" => Self::Text,
            "application" => Self::Application,
            "message" => Self::Message,
            "image" => Self::Image,
            other => Self::Other(other.to_owned()),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct SDPRangedPort {
    pub port: u16,
    pub range: Option<u16>,
}

impl From<u16> for SDPRangedPort {
    fn from(value: u16) -> Self {
        SDPRangedPort {
            port: value,
            range: None,
        }
    }
}

impl fmt::Display for SDPRangedPort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.port)?;
        if let Some(range) = self.range {
            write!(f, "/{}", range)?;
        }
        Ok(())
    }
}

impl TryFrom<&str> for SDPRangedPort {
    type Error = SDPError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if !value.contains('/') {
            return Ok(Self {
                port: value.parse().map_err(|err| {
                    SDPError::SyntaxError(format!("parse port failed: {}, {}", value, err))
                })?,
                range: None,
            });
        }

        let fields: Vec<&str> = value.split('/').collect();
        if fields.len() != 2 {
            return Err(SDPError::SyntaxError("invalid ranged port".to_owned()));
        }

        Ok(Self {
            port: fields[0].parse().map_err(|err| {
                SDPError::SyntaxError(format!("parse port failed: {}, {}", fields[0], err))
            })?,
            range: Some(fields[1].parse().map_err(|err| {
                SDPError::SyntaxError(format!("parse port range failed: {}, {}", fields[1], err))
            })?),
        })
    }
}

#[derive(Debug, Default, Clone)]
pub enum SDPMediaProtocol {
    #[default]
    UDP, // udp
    RtpAvp,   // RTP/AVP
    RtpSAvp,  // RTP/SAVP
    RtpSAvpF, // RTP/SAVPF
    Other(String),
}

impl fmt::Display for SDPMediaProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::UDP => "udp",
                Self::RtpAvp => "RTP/AVP",
                Self::RtpSAvp => "RTP/SAVP",
                Self::RtpSAvpF => "RTP/SAVPF",
                Self::Other(str) => str,
            }
        )
    }
}

impl From<&str> for SDPMediaProtocol {
    fn from(value: &str) -> Self {
        match value {
            "udp" => Self::UDP,
            "RTP/AVP" => Self::RtpAvp,
            "RTP/SAVP" => Self::RtpSAvp,
            "RTP/SAVPF" => Self::RtpSAvpF,
            other => Self::Other(other.to_owned()),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct SDPMediaLine {
    pub media_type: SDPMediaType,
    pub port: SDPRangedPort,
    pub protocol: SDPMediaProtocol,
    pub format: Vec<String>,
}

impl fmt::Display for SDPMediaLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "m={} {} {}", self.media_type, self.port, self.protocol)?;
        self.format
            .iter()
            .try_for_each(|item| write!(f, " {}", item))?;
        write!(f, "{}", CRLF)
    }
}

#[derive(Debug, Default, Clone)]
pub struct SDPMediaDescription {
    pub media_line: SDPMediaLine,
    pub media_title: Option<SDPSessionInformation>,
    pub connection_information: Vec<SDPConnectionInformation>,
    pub bandwidth: Vec<SDPBandWidthInformation>,
    pub encryption_key: Option<SDPEncryptionKeys>,
    pub attributes: Vec<SDPAttribute>,
}

impl SDPMediaDescription {
    pub fn get_rtp_map(&self) -> Option<RtpMap> {
        self.attributes.iter().find_map(|attr| {
            if let SDPAttribute::RtpMap(rtpmap) = attr {
                Some(rtpmap.clone())
            } else {
                None
            }
        })
    }
    pub fn get_fmtp(&self) -> Option<FormatParameters> {
        self.attributes.iter().find_map(|attr| {
            if let SDPAttribute::Fmtp(fmtp) = attr {
                Some(fmtp.clone())
            } else {
                None
            }
        })
    }
}

impl fmt::Display for SDPMediaDescription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.media_line)?;
        if let Some(title) = &self.media_title {
            write!(f, "i={}{}", title, CRLF)?;
        }
        self.connection_information
            .iter()
            .try_for_each(|item| write!(f, "{}", item))?;
        self.bandwidth
            .iter()
            .try_for_each(|item| write!(f, "{}", item))?;
        if let Some(key) = &self.encryption_key {
            write!(f, "{}", key)?;
        }
        self.attributes
            .iter()
            .try_for_each(|item| write!(f, "a={}{}", item, CRLF))?;
        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct Sdp {
    pub version: SDPVersion,
    pub origin: SDPOrigin,
    pub session_name: SDPSessionName,
    pub session_information: Option<SDPSessionInformation>,
    pub uri: Option<SDPUri>,
    pub email_address: Vec<SDPEmail>,
    pub phone_number: Vec<SDPPhoneNumber>,
    pub connection_information: Option<SDPConnectionInformation>,
    pub bandwidth_information: Vec<SDPBandWidthInformation>,
    pub time_information: Vec<SDPTimeInformation>,

    pub encryption_keys: Option<SDPEncryptionKeys>,
    pub attributes: Vec<SDPAttribute>,
    pub media_description: Vec<SDPMediaDescription>,
}

impl Sdp {
    pub fn reader() -> SessionDescriptionReader {
        SessionDescriptionReader::new()
    }
}

impl<R: io::BufRead> ReadFrom<R> for Sdp {
    type Error = SDPError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let mut text = String::new();
        reader.read_to_string(&mut text)?;
        Self::reader().read_from(&text)
    }
}

impl fmt::Display for Sdp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v={}{}", self.version, CRLF)?;
        write!(f, "{}", self.origin)?;
        write!(f, "s={}{}", self.session_name, CRLF)?;
        if let Some(info) = &self.session_information {
            write!(f, "i={}{}", info, CRLF)?;
        }
        if let Some(u) = &self.uri {
            write!(f, "u={}{}", u, CRLF)?;
        }
        self.email_address
            .iter()
            .try_for_each(|item| write!(f, "e={}{}", item, CRLF))?;
        self.phone_number
            .iter()
            .try_for_each(|item| write!(f, "p={}{}", item, CRLF))?;
        if let Some(conn) = &self.connection_information {
            write!(f, "{}", conn)?;
        }
        self.bandwidth_information
            .iter()
            .try_for_each(|item| write!(f, "{}", item))?;
        self.time_information
            .iter()
            .try_for_each(|item| write!(f, "{}", item))?;

        if let Some(key) = &self.encryption_keys {
            write!(f, "{}", key)?;
        }

        self.attributes
            .iter()
            .try_for_each(|item| write!(f, "a={}{}", item, CRLF))?;
        self.media_description
            .iter()
            .try_for_each(|item| write!(f, "{}", item))?;
        Ok(())
    }
}
