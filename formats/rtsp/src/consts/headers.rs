//! @see: RFC 7826 Table 1
use std::fmt;

use crate::errors::RTSPMessageError;

pub mod header_names {
    pub const ACCEPT: &str = "Accept";
    pub const ACCEPT_CREDENTIALS: &str = "Accept-Credentials";
    pub const ACCEPT_ENCODING: &str = "Accept-Encoding";
    pub const ACCEPT_LANGUAGE: &str = "Accept-Language";
    pub const ACCEPT_RANGES: &str = "Accept-Ranges";
    pub const ALLOW: &str = "Allow";
    pub const AUTHENTICATION_INFO: &str = "Authentication-Info";
    pub const AUTHORIZATION: &str = "Authorization";

    pub const BANDWIDTH: &str = "Bandwidth";
    pub const BLOCKSIZE: &str = "Blocksize";

    pub const CACHE_CONTROL: &str = "Cache-Control";
    pub const CONNECTION: &str = "Connection";
    pub const CONNECTION_CREDENTIALS: &str = "Connection-Credentials";
    pub const CONTENT_BASE: &str = "Content-Base";
    pub const CONTENT_ENCODING: &str = "Content-Encoding";
    pub const CONTENT_LANGUAGE: &str = "Content-Language";
    pub const CONTENT_LENGTH: &str = "Content-Length";
    pub const CONTENT_LOCATION: &str = "Content-Location";
    pub const CONTENT_TYPE: &str = "Content-Type";
    pub const C_SEQ: &str = "CSeq";

    pub const DATE: &str = "Date";

    pub const EXPIRES: &str = "Expires";

    pub const FROM: &str = "From";

    pub const IF_MATCH: &str = "If-Match";
    pub const IF_MODIFIED_SINCE: &str = "If-Modified-Since";
    pub const IF_NONE_MATCH: &str = "If-None-Match";

    pub const LAST_MODIFIED: &str = "Last-Modified";
    pub const LOCATION: &str = "Location";

    pub const MEDIA_PROPERTIES: &str = "Media-Properties";
    pub const MEDIA_RANGE: &str = "Media-Range";
    pub const M_TAG: &str = "MTag";

    pub const NOTIFY_REASON: &str = "Notify-Reason";

    pub const PIPELINED_REQUESTS: &str = "Pipelined-Requests";
    pub const PROXY_AUTHENTICATE: &str = "Proxy-Authenticate";
    pub const PROXY_AUTHENTICATION_INFO: &str = "Proxy-Authentication-Info";
    pub const PROXY_AUTHORIZATION: &str = "Proxy-Authorization";
    pub const PROXY_REQUIRE: &str = "Proxy-Require";
    pub const PROXY_SUPPORTED: &str = "Proxy-Supported";
    pub const PUBLIC: &str = "Public";

    pub const RANGE: &str = "Range";
    pub const REFERRER: &str = "Referrer";
    pub const REQUEST_STATUS: &str = "Request-Status";
    pub const REQUIRE: &str = "Require";
    pub const RETRY_AFTER: &str = "Retry-After";
    pub const RTP_INFO: &str = "RTP-Info";

    pub const SCALE: &str = "Scale";
    pub const SEEK_STYLE: &str = "Seek-Style";
    pub const SERVER: &str = "Server";
    pub const SESSION: &str = "Session";
    pub const SPEED: &str = "Speed";
    pub const SUPPORTED: &str = "Supported";

    pub const TERMINATE_REASON: &str = "Terminate-Reason";
    pub const TIMESTAMP: &str = "Timestamp";
    pub const TRANSPORT: &str = "Transport";

    pub const UNSUPPORTED: &str = "Unsupported";
    pub const USER_AGENT: &str = "User-Agent";

    pub const VIA: &str = "Via";

    pub const WWW_AUTHENTICATE: &str = "WWW-Authenticate";
}

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

impl TryFrom<&str> for RtspHeader {
    type Error = RTSPMessageError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
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

            _ => Err(RTSPMessageError::UnknownHeader(Some(value.into()))),
        }
    }
}
