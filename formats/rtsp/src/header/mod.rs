pub mod header_names;
pub mod transport;
use std::{
    fmt,
    io::{self, Read},
    str::FromStr,
};

use tokio_util::bytes::Buf;
use transport::TransportHeader;
use utils::traits::reader::{ReadFrom, TryReadFrom};

use crate::{consts::common::CRLF_STR, errors::RtspMessageError, util::TextReader};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtspHeader {
    Accept,
    AcceptCredentials,
    AcceptEncoding,
    AcceptLanguage,
    AcceptRanges,
    Allow,
    AuthenticationInfo,
    Authorization,

    Bandwidth,
    Blocksize,

    CacheControl,
    Connection,
    ConnectionCredentials,
    ContentBase,
    ContentEncoding,
    ContentLanguage,
    ContentLength,
    ContentLocation,
    ContentType,
    CSeq,

    Date,

    Expires,

    From,

    IfMatch,
    IfModifiedSince,
    IfNoneMatch,

    LastModified,
    Location,

    MediaProperties,
    MediaRange,
    MTag,

    NotifyReason,

    PipelinedRequests,
    ProxyAuthenticate,
    ProxyAuthenticationInfo,
    ProxyAuthorization,
    ProxyRequire,
    ProxySupported,
    Public,

    Range,
    Referrer,
    RequestStatus,
    Require,
    RetryAfter,
    RtpInfo,

    Scale,
    SeekStyle,
    Server,
    Session,
    Speed,
    Supported,

    TerminateReason,
    Timestamp,
    Transport,

    Unsupported,
    UserAgent,

    Via,

    WWWAuthenticate,
}

impl From<&RtspHeader> for &str {
    fn from(value: &RtspHeader) -> Self {
        match value {
            RtspHeader::Accept => header_names::ACCEPT,
            RtspHeader::AcceptCredentials => header_names::ACCEPT_CREDENTIALS,
            RtspHeader::AcceptEncoding => header_names::ACCEPT_ENCODING,
            RtspHeader::AcceptLanguage => header_names::ACCEPT_LANGUAGE,
            RtspHeader::AcceptRanges => header_names::ACCEPT_RANGES,
            RtspHeader::Allow => header_names::ALLOW,
            RtspHeader::AuthenticationInfo => header_names::AUTHENTICATION_INFO,
            RtspHeader::Authorization => header_names::AUTHORIZATION,

            RtspHeader::Bandwidth => header_names::BANDWIDTH,
            RtspHeader::Blocksize => header_names::BLOCKSIZE,

            RtspHeader::CacheControl => header_names::CACHE_CONTROL,
            RtspHeader::Connection => header_names::CONNECTION,
            RtspHeader::ConnectionCredentials => header_names::CONNECTION_CREDENTIALS,
            RtspHeader::ContentBase => header_names::CONTENT_BASE,
            RtspHeader::ContentEncoding => header_names::CONTENT_ENCODING,
            RtspHeader::ContentLanguage => header_names::CONTENT_LANGUAGE,
            RtspHeader::ContentLength => header_names::CONTENT_LENGTH,
            RtspHeader::ContentLocation => header_names::CONTENT_LOCATION,
            RtspHeader::ContentType => header_names::CONTENT_TYPE,
            RtspHeader::CSeq => header_names::C_SEQ,

            RtspHeader::Date => header_names::DATE,

            RtspHeader::Expires => header_names::EXPIRES,

            RtspHeader::From => header_names::FROM,

            RtspHeader::IfMatch => header_names::IF_MATCH,
            RtspHeader::IfModifiedSince => header_names::IF_MODIFIED_SINCE,
            RtspHeader::IfNoneMatch => header_names::IF_NONE_MATCH,

            RtspHeader::LastModified => header_names::LAST_MODIFIED,
            RtspHeader::Location => header_names::LOCATION,

            RtspHeader::MediaProperties => header_names::MEDIA_PROPERTIES,
            RtspHeader::MediaRange => header_names::MEDIA_RANGE,
            RtspHeader::MTag => header_names::M_TAG,

            RtspHeader::NotifyReason => header_names::NOTIFY_REASON,

            RtspHeader::PipelinedRequests => header_names::PIPELINED_REQUESTS,
            RtspHeader::ProxyAuthenticate => header_names::PROXY_AUTHENTICATE,
            RtspHeader::ProxyAuthenticationInfo => header_names::PROXY_AUTHENTICATION_INFO,
            RtspHeader::ProxyAuthorization => header_names::PROXY_AUTHORIZATION,
            RtspHeader::ProxyRequire => header_names::PROXY_REQUIRE,
            RtspHeader::ProxySupported => header_names::PROXY_SUPPORTED,
            RtspHeader::Public => header_names::PUBLIC,

            RtspHeader::Range => header_names::RANGE,
            RtspHeader::Referrer => header_names::REFERRER,
            RtspHeader::RequestStatus => header_names::REQUEST_STATUS,
            RtspHeader::Require => header_names::REQUIRE,
            RtspHeader::RetryAfter => header_names::RETRY_AFTER,
            RtspHeader::RtpInfo => header_names::RTP_INFO,

            RtspHeader::Scale => header_names::SCALE,
            RtspHeader::SeekStyle => header_names::SEEK_STYLE,
            RtspHeader::Server => header_names::SERVER,
            RtspHeader::Session => header_names::SESSION,
            RtspHeader::Speed => header_names::SPEED,
            RtspHeader::Supported => header_names::SUPPORTED,

            RtspHeader::TerminateReason => header_names::TERMINATE_REASON,
            RtspHeader::Timestamp => header_names::TIMESTAMP,
            RtspHeader::Transport => header_names::TRANSPORT,

            RtspHeader::Unsupported => header_names::UNSUPPORTED,
            RtspHeader::UserAgent => header_names::USER_AGENT,

            RtspHeader::Via => header_names::VIA,

            RtspHeader::WWWAuthenticate => header_names::WWW_AUTHENTICATE,
        }
    }
}

impl fmt::Display for RtspHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str: &str = self.into();
        f.write_str(str)
    }
}

impl FromStr for RtspHeader {
    type Err = RtspMessageError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            header_names::ACCEPT => Ok(Self::Accept),
            header_names::ACCEPT_CREDENTIALS => Ok(Self::AcceptCredentials),
            header_names::ACCEPT_ENCODING => Ok(Self::AcceptEncoding),
            header_names::ACCEPT_LANGUAGE => Ok(Self::AcceptLanguage),
            header_names::ACCEPT_RANGES => Ok(Self::AcceptRanges),
            header_names::ALLOW => Ok(Self::Allow),
            header_names::AUTHENTICATION_INFO => Ok(Self::AuthenticationInfo),
            header_names::AUTHORIZATION => Ok(Self::Authorization),

            header_names::BANDWIDTH => Ok(Self::Bandwidth),
            header_names::BLOCKSIZE => Ok(Self::Blocksize),

            header_names::CACHE_CONTROL => Ok(Self::CacheControl),
            header_names::CONNECTION => Ok(Self::Connection),
            header_names::CONNECTION_CREDENTIALS => Ok(Self::ConnectionCredentials),
            header_names::CONTENT_BASE => Ok(Self::ContentBase),
            header_names::CONTENT_ENCODING => Ok(Self::ContentEncoding),
            header_names::CONTENT_LANGUAGE => Ok(Self::ContentLanguage),
            header_names::CONTENT_LENGTH => Ok(Self::ContentLength),
            header_names::CONTENT_LOCATION => Ok(Self::ContentLocation),
            header_names::CONTENT_TYPE => Ok(Self::ContentType),
            header_names::C_SEQ => Ok(Self::CSeq),

            header_names::DATE => Ok(Self::Date),

            header_names::EXPIRES => Ok(Self::Expires),

            header_names::FROM => Ok(Self::From),

            header_names::IF_MATCH => Ok(Self::IfMatch),
            header_names::IF_MODIFIED_SINCE => Ok(Self::IfModifiedSince),
            header_names::IF_NONE_MATCH => Ok(Self::IfNoneMatch),

            header_names::LAST_MODIFIED => Ok(Self::LastModified),
            header_names::LOCATION => Ok(Self::Location),

            header_names::MEDIA_PROPERTIES => Ok(Self::MediaProperties),
            header_names::MEDIA_RANGE => Ok(Self::MediaRange),
            header_names::M_TAG => Ok(Self::MTag),

            header_names::NOTIFY_REASON => Ok(Self::NotifyReason),

            header_names::PIPELINED_REQUESTS => Ok(Self::PipelinedRequests),
            header_names::PROXY_AUTHENTICATE => Ok(Self::ProxyAuthenticate),
            header_names::PROXY_AUTHENTICATION_INFO => Ok(Self::ProxyAuthenticationInfo),
            header_names::PROXY_AUTHORIZATION => Ok(Self::ProxyAuthorization),
            header_names::PROXY_REQUIRE => Ok(Self::ProxyRequire),
            header_names::PROXY_SUPPORTED => Ok(Self::ProxySupported),
            header_names::PUBLIC => Ok(Self::Public),

            header_names::RANGE => Ok(Self::Range),
            header_names::REFERRER => Ok(Self::Referrer),
            header_names::REQUEST_STATUS => Ok(Self::RequestStatus),
            header_names::REQUIRE => Ok(Self::Require),
            header_names::RETRY_AFTER => Ok(Self::RetryAfter),
            header_names::RTP_INFO => Ok(Self::RtpInfo),

            header_names::SCALE => Ok(Self::Scale),
            header_names::SEEK_STYLE => Ok(Self::SeekStyle),
            header_names::SERVER => Ok(Self::Server),
            header_names::SESSION => Ok(Self::Session),
            header_names::SPEED => Ok(Self::Speed),
            header_names::SUPPORTED => Ok(Self::Supported),

            header_names::TERMINATE_REASON => Ok(Self::TerminateReason),
            header_names::TIMESTAMP => Ok(Self::Timestamp),
            header_names::TRANSPORT => Ok(Self::Transport),

            header_names::UNSUPPORTED => Ok(Self::Unsupported),
            header_names::USER_AGENT => Ok(Self::UserAgent),

            header_names::VIA => Ok(Self::Via),

            header_names::WWW_AUTHENTICATE => Ok(Self::WWWAuthenticate),

            _ => Err(RtspMessageError::UnknownHeader(Some(s.into()))),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct RtspHeaders(Vec<(RtspHeader, String)>);

impl RtspHeaders {
    pub fn new(items: Vec<(RtspHeader, String)>) -> Self {
        Self(items)
    }
    pub fn push<S: Into<String>>(&mut self, key: RtspHeader, value: S) {
        self.0.push((key, value.into()));
    }

    pub fn append(&mut self, mut items: Vec<(RtspHeader, String)>) {
        self.0.append(&mut items);
    }

    pub fn get(&self, key: RtspHeader) -> Vec<&String> {
        self.0
            .iter()
            .filter(|(k, _)| k.eq(&key))
            .map(|(_, value)| value)
            .collect()
    }

    pub fn get_unique(&self, key: RtspHeader) -> Option<&String> {
        self.get(key).first().copied()
    }

    pub fn contains(&self, key: RtspHeader) -> bool {
        self.0.iter().any(|(k, _)| k.eq(&key))
    }

    pub fn remove(&mut self, key: RtspHeader) {
        self.0.retain(|(k, _)| k.ne(&key));
    }

    pub fn entries(&self) -> &Vec<(RtspHeader, String)> {
        &self.0
    }

    pub fn entries_mut(&mut self) -> &mut Vec<(RtspHeader, String)> {
        &mut self.0
    }

    pub fn set<S: Into<String>>(&mut self, key: RtspHeader, value: S) {
        self.remove(key);
        self.push(key, value.into());
    }

    pub fn cseq(&self) -> Option<u32> {
        self.get_unique(RtspHeader::CSeq)
            .and_then(|cseq| cseq.parse().ok())
    }

    pub fn transport(&self) -> Option<TransportHeader> {
        self.get_unique(RtspHeader::Transport)
            .and_then(|trans| trans.parse().ok())
    }
}

impl fmt::Display for RtspHeaders {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.entries().iter().try_for_each(|(key, value)| {
            f.write_fmt(format_args!("{}: {}{}", key, value, CRLF_STR))
        })
    }
}

impl<R: io::BufRead> ReadFrom<R> for RtspHeaders {
    type Error = RtspMessageError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let buffer = reader.fill_buf()?.to_vec();
        let mut cursor = io::Cursor::new(&buffer);
        if let Some(headers) = Self::try_read_from(cursor.by_ref())? {
            reader.consume(cursor.position() as usize);
            return Ok(headers);
        }
        Err(RtspMessageError::InvalidRtspMessageFormat(format!(
            "the message is incomplete: {}",
            String::from_utf8_lossy(&buffer),
        )))
    }
}

impl<R: AsRef<[u8]>> TryReadFrom<R> for RtspHeaders {
    type Error = RtspMessageError;
    fn try_read_from(reader: &mut io::Cursor<R>) -> Result<Option<Self>, Self::Error> {
        if !reader.has_remaining() {
            return Ok(None);
        }
        let mut text_reader = TextReader::new(reader.by_ref());
        let mut headers = vec![];
        loop {
            let line = text_reader.read_line()?;
            if line.is_none() {
                // at least CRLF should be there
                return Ok(None);
            }

            let line = line.unwrap();
            let trimmed_line = line.trim();
            if trimmed_line.is_empty() {
                break;
            }
            let parts: Vec<_> = trimmed_line.split(":").collect();
            if parts.len() < 2 {
                return Err(RtspMessageError::InvalidRtspMessageFormat(format!(
                    "invalid header line: {}",
                    line
                )));
            }

            let key = parts[0].parse()?;
            let value = parts[1..].join(":");
            headers.push((key, value.trim().to_owned()));
        }

        Ok(Some(Self(headers)))
    }
}
