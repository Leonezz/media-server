//! @see: RFC 7826 Table 1
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
pub enum PredefinedRtspHeader {
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

impl From<PredefinedRtspHeader> for &str {
    fn from(value: PredefinedRtspHeader) -> Self {
        match value {
            PredefinedRtspHeader::Accept => header_names::ACCEPT,
            PredefinedRtspHeader::AcceptCredentials => header_names::ACCEPT_CREDENTIALS,
            PredefinedRtspHeader::AcceptEncoding => header_names::ACCEPT_ENCODING,
            PredefinedRtspHeader::AcceptLanguage => header_names::ACCEPT_LANGUAGE,
            PredefinedRtspHeader::AcceptRanges => header_names::ACCEPT_RANGES,
            PredefinedRtspHeader::Allow => header_names::ALLOW,
            PredefinedRtspHeader::AuthenticationInfo => header_names::AUTHENTICATION_INFO,
            PredefinedRtspHeader::Authorization => header_names::AUTHORIZATION,

            PredefinedRtspHeader::Bandwidth => header_names::BANDWIDTH,
            PredefinedRtspHeader::Blocksize => header_names::BLOCKSIZE,

            PredefinedRtspHeader::CacheControl => header_names::CACHE_CONTROL,
            PredefinedRtspHeader::Connection => header_names::CONNECTION,
            PredefinedRtspHeader::ConnectionCredentials => header_names::CONNECTION_CREDENTIALS,
            PredefinedRtspHeader::ContentBase => header_names::CONTENT_BASE,
            PredefinedRtspHeader::ContentEncoding => header_names::CONTENT_ENCODING,
            PredefinedRtspHeader::ContentLanguage => header_names::CONTENT_LANGUAGE,
            PredefinedRtspHeader::ContentLength => header_names::CONTENT_LENGTH,
            PredefinedRtspHeader::ContentLocation => header_names::CONTENT_LOCATION,
            PredefinedRtspHeader::ContentType => header_names::CONTENT_TYPE,
            PredefinedRtspHeader::CSeq => header_names::C_SEQ,

            PredefinedRtspHeader::Date => header_names::DATE,

            PredefinedRtspHeader::Expires => header_names::EXPIRES,

            PredefinedRtspHeader::From => header_names::FROM,

            PredefinedRtspHeader::IfMatch => header_names::IF_MATCH,
            PredefinedRtspHeader::IfModifiedSince => header_names::IF_MODIFIED_SINCE,
            PredefinedRtspHeader::IfNoneMatch => header_names::IF_NONE_MATCH,

            PredefinedRtspHeader::LastModified => header_names::LAST_MODIFIED,
            PredefinedRtspHeader::Location => header_names::LOCATION,

            PredefinedRtspHeader::MediaProperties => header_names::MEDIA_PROPERTIES,
            PredefinedRtspHeader::MediaRange => header_names::MEDIA_RANGE,
            PredefinedRtspHeader::MTag => header_names::M_TAG,

            PredefinedRtspHeader::NotifyReason => header_names::NOTIFY_REASON,

            PredefinedRtspHeader::PipelinedRequests => header_names::PIPELINED_REQUESTS,
            PredefinedRtspHeader::ProxyAuthenticate => header_names::PROXY_AUTHENTICATE,
            PredefinedRtspHeader::ProxyAuthenticationInfo => {
                header_names::PROXY_AUTHENTICATION_INFO
            }
            PredefinedRtspHeader::ProxyAuthorization => header_names::PROXY_AUTHORIZATION,
            PredefinedRtspHeader::ProxyRequire => header_names::PROXY_REQUIRE,
            PredefinedRtspHeader::ProxySupported => header_names::PROXY_SUPPORTED,
            PredefinedRtspHeader::Public => header_names::PUBLIC,

            PredefinedRtspHeader::Range => header_names::RANGE,
            PredefinedRtspHeader::Referrer => header_names::REFERRER,
            PredefinedRtspHeader::RequestStatus => header_names::REQUEST_STATUS,
            PredefinedRtspHeader::Require => header_names::REQUIRE,
            PredefinedRtspHeader::RetryAfter => header_names::RETRY_AFTER,
            PredefinedRtspHeader::RtpInfo => header_names::RTP_INFO,

            PredefinedRtspHeader::Scale => header_names::SCALE,
            PredefinedRtspHeader::SeekStyle => header_names::SEEK_STYLE,
            PredefinedRtspHeader::Server => header_names::SERVER,
            PredefinedRtspHeader::Session => header_names::SESSION,
            PredefinedRtspHeader::Speed => header_names::SPEED,
            PredefinedRtspHeader::Supported => header_names::SUPPORTED,

            PredefinedRtspHeader::TerminateReason => header_names::TERMINATE_REASON,
            PredefinedRtspHeader::Timestamp => header_names::TIMESTAMP,
            PredefinedRtspHeader::Transport => header_names::TRANSPORT,

            PredefinedRtspHeader::Unsupported => header_names::UNSUPPORTED,
            PredefinedRtspHeader::UserAgent => header_names::USER_AGENT,

            PredefinedRtspHeader::Via => header_names::VIA,

            PredefinedRtspHeader::WWWAuthenticate => header_names::WWW_AUTHENTICATE,
        }
    }
}

impl TryFrom<&str> for PredefinedRtspHeader {
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
            header_names::CONTENT_LENGTH => Ok(Self::ContentType),
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

            _ => Err(RTSPMessageError::UnknownHeader(value.into())),
        }
    }
}
