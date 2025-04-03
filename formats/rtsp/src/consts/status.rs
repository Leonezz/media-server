//! @ses: RFC 7826 Table 4
use std::fmt::Display;

use crate::errors::RTSPMessageError;

pub mod status_description {
    pub const CONTINUE: &str = "Continue";
    pub const OK: &str = "OK";
    pub const MOVED_PERMANENTLY: &str = "Moved Permanently";
    pub const FOUND: &str = "Found";
    pub const SEE_OTHER: &str = "See Other";
    pub const NOT_MODIFIED: &str = "Not Modified";
    pub const USE_PROXY: &str = "Use Proxy";
    pub const BAD_REQUEST: &str = "Bad Request";
    pub const UNAUTHORIZED: &str = "Unauthorized";
    pub const PAYMENT_REQUIRED: &str = "Payment Required";
    pub const FORBIDDEN: &str = "Forbidden";
    pub const NOT_FOUND: &str = "Not Found";
    pub const METHOD_NOT_ALLOWED: &str = "Method Not Allowed";
    pub const NOT_ACCEPTABLE: &str = "Not Acceptable";
    pub const PROXY_AUTHENTICATION_REQUIRED: &str = "Proxy Authentication Required";
    pub const REQUEST_TIMEOUT: &str = "Request Timeout";
    pub const GONE: &str = "Gone";
    pub const PRECONDITION_FAILED: &str = "Precondition Failed";
    pub const REQUEST_MESSAGE_BODY_TOO_LARGE: &str = "Request Message Body Too Large";
    pub const REQUEST_URI_TOO_LONG: &str = "Request-URI Too Long";
    pub const UNSUPPORTED_MEDIA_TYPE: &str = "Unsupported Media Type";
    pub const PARAMETER_NOT_UNDERSTOOD: &str = "Parameter Not Understood";
    pub const RESERVED: &str = "reserved";
    pub const NOT_ENOUGH_BANDWIDTH: &str = "Not Enough Bandwidth";
    pub const SESSION_NOT_FOUND: &str = "Session Not Found";
    pub const METHOD_NOT_VALID_IN_THIS_STATE: &str = "Method Not Valid in This State";
    pub const HEADER_FIELD_NOT_VALID_FOR_RESOURCE: &str = "Header Field Not Valid for Resource";
    pub const INVALID_RANGE: &str = "Invalid Range";
    pub const PARAMETER_IS_READ_ONLY: &str = "Parameter Is Read-Only";
    pub const AGGREGATE_OPERATION_NOT_ALLOWED: &str = "Aggregate Operation Not Allowed";
    pub const ONLY_AGGREGATE_OPERATION_ALLOWED: &str = "Only Aggregate Operation Allowed";
    pub const UNSUPPORTED_TRANSPORT: &str = "Unsupported Transport";
    pub const DESTINATION_UNREACHABLE: &str = "Destination Unreachable";
    pub const DESTINATION_PROHIBITED: &str = "Destination Prohibited";
    pub const DATA_TRANSPORT_NOT_READY_YET: &str = "Data Transport Not Ready Yet";
    pub const NOTIFICATION_REASON_UNKNOWN: &str = "Notification Reason Unknown";
    pub const KEY_MANAGEMENT_ERROR: &str = "Key Management Error";
    pub const CONNECTION_AUTHORIZATION_REQUIRED: &str = "Connection Authorization Required";
    pub const CONNECTION_CREDENTIALS_NOT_ACCEPTED: &str = "Connection Credentials Not Accepted";
    pub const FAILURE_TO_ESTABLISH_SECURE_CONNECTION: &str =
        "Failure to Establish Secure Connection";
    pub const INTERNAL_SERVER_ERROR: &str = "Internal Server Error";
    pub const NOT_IMPLEMENTED: &str = "Not Implemented";
    pub const BAD_GATEWAY: &str = "Bad Gateway";
    pub const SERVICE_UNAVAILABLE: &str = "Service Unavailable";
    pub const GATEWAY_TIMEOUT: &str = "Gateway Timeout";
    pub const RTSP_VERSION_NOT_SUPPORTED: &str = "RTSP Version Not Supported";
    pub const OPTION_NOT_SUPPORTED: &str = "Option Not Supported";
    pub const PROXY_UNAVAILABLE: &str = "Proxy Unavailable";
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtspStatus {
    Continue = 100,
    OK = 200,
    MovedPermanently = 301,
    Found = 302,
    SeeOther = 303,
    NotModified = 304,
    UseProxy = 305,
    BadRequest = 400,
    Unauthorized = 401,
    PaymentRequired = 402,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    NotAcceptable = 406,
    ProxyAuthenticationRequired = 407,
    RequestTimeout = 408,
    Gone = 410,
    PreconditionFailed = 412,
    RequestMessageBodyTooLarge = 413,
    RequestUriTooLong = 414,
    UnsupportedMediaType = 415,
    ParameterNotUnderstood = 451,
    Reserved = 452,
    NotEnoughBandwidth = 453,
    SessionNotFound = 454,
    MethodNotValidInThisState = 455,
    HeaderFieldNotValidForResource = 456,
    InvalidRange = 457,
    ParameterIsReadOnly = 458,
    AggregateOperationNotAllowed = 459,
    OnlyAggregateOperationAllowed = 460,
    UnsupportedTransport = 461,
    DestinationUnreachable = 462,
    DestinationProhibited = 463,
    DataTransportNotReadyYet = 464,
    NotificationReasonUnknown = 465,
    KeyManagementError = 466,
    ConnectionAuthorizationRequired = 470,
    ConnectionCredentialsNotAccepted = 471,
    FailureToEstablishSecureConnection = 472,
    InternalServerError = 500,
    NotImplemented = 501,
    BadGateWay = 502,
    ServiceUnavailable = 503,
    GatewayTimeout = 504,
    RtspVersionNotSupported = 505,
    OptionNotSupported = 551,
    ProxyUnavailable = 553,
}

impl From<RtspStatus> for u16 {
    fn from(value: RtspStatus) -> Self {
        value as u16
    }
}

impl TryFrom<u16> for RtspStatus {
    type Error = RTSPMessageError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            100 => Ok(Self::Continue),
            200 => Ok(Self::OK),
            301 => Ok(Self::MovedPermanently),
            302 => Ok(Self::Found),
            303 => Ok(Self::SeeOther),
            304 => Ok(Self::NotModified),
            305 => Ok(Self::UseProxy),
            400 => Ok(Self::BadRequest),
            401 => Ok(Self::Unauthorized),
            402 => Ok(Self::PaymentRequired),
            403 => Ok(Self::Forbidden),
            404 => Ok(Self::NotFound),
            405 => Ok(Self::MethodNotAllowed),
            406 => Ok(Self::NotAcceptable),
            407 => Ok(Self::ProxyAuthenticationRequired),
            408 => Ok(Self::RequestTimeout),
            410 => Ok(Self::Gone),
            412 => Ok(Self::PreconditionFailed),
            413 => Ok(Self::RequestMessageBodyTooLarge),
            414 => Ok(Self::RequestUriTooLong),
            415 => Ok(Self::UnsupportedMediaType),
            451 => Ok(Self::ParameterNotUnderstood),
            452 => Ok(Self::Reserved),
            453 => Ok(Self::NotEnoughBandwidth),
            454 => Ok(Self::SessionNotFound),
            455 => Ok(Self::MethodNotValidInThisState),
            456 => Ok(Self::HeaderFieldNotValidForResource),
            457 => Ok(Self::InvalidRange),
            458 => Ok(Self::ParameterIsReadOnly),
            459 => Ok(Self::AggregateOperationNotAllowed),
            460 => Ok(Self::OnlyAggregateOperationAllowed),
            461 => Ok(Self::UnsupportedTransport),
            462 => Ok(Self::DestinationUnreachable),
            463 => Ok(Self::DestinationProhibited),
            464 => Ok(Self::DataTransportNotReadyYet),
            465 => Ok(Self::NotificationReasonUnknown),
            466 => Ok(Self::KeyManagementError),
            470 => Ok(Self::ConnectionAuthorizationRequired),
            471 => Ok(Self::ConnectionCredentialsNotAccepted),
            472 => Ok(Self::FailureToEstablishSecureConnection),
            500 => Ok(Self::InternalServerError),
            501 => Ok(Self::NotImplemented),
            502 => Ok(Self::BadGateWay),
            503 => Ok(Self::ServiceUnavailable),
            504 => Ok(Self::GatewayTimeout),
            505 => Ok(Self::RtspVersionNotSupported),
            551 => Ok(Self::OptionNotSupported),
            553 => Ok(Self::ProxyUnavailable),
            _ => Err(RTSPMessageError::UnknownStatusCode(Some(value))),
        }
    }
}

impl From<RtspStatus> for &'static str {
    fn from(val: RtspStatus) -> Self {
        match val {
            RtspStatus::Continue => status_description::CONTINUE,
            RtspStatus::OK => status_description::OK,
            RtspStatus::MovedPermanently => status_description::MOVED_PERMANENTLY,
            RtspStatus::Found => status_description::FOUND,
            RtspStatus::SeeOther => status_description::SEE_OTHER,
            RtspStatus::NotModified => status_description::NOT_MODIFIED,
            RtspStatus::UseProxy => status_description::USE_PROXY,
            RtspStatus::BadRequest => status_description::BAD_REQUEST,
            RtspStatus::Unauthorized => status_description::UNAUTHORIZED,
            RtspStatus::PaymentRequired => status_description::PAYMENT_REQUIRED,
            RtspStatus::Forbidden => status_description::FORBIDDEN,
            RtspStatus::NotFound => status_description::NOT_FOUND,
            RtspStatus::MethodNotAllowed => status_description::METHOD_NOT_ALLOWED,
            RtspStatus::NotAcceptable => status_description::NOT_ACCEPTABLE,
            RtspStatus::ProxyAuthenticationRequired => {
                status_description::PROXY_AUTHENTICATION_REQUIRED
            }
            RtspStatus::RequestTimeout => status_description::REQUEST_TIMEOUT,
            RtspStatus::Gone => status_description::GONE,
            RtspStatus::PreconditionFailed => status_description::PRECONDITION_FAILED,
            RtspStatus::RequestMessageBodyTooLarge => {
                status_description::REQUEST_MESSAGE_BODY_TOO_LARGE
            }
            RtspStatus::RequestUriTooLong => status_description::REQUEST_URI_TOO_LONG,
            RtspStatus::UnsupportedMediaType => status_description::UNSUPPORTED_MEDIA_TYPE,
            RtspStatus::ParameterNotUnderstood => status_description::PARAMETER_NOT_UNDERSTOOD,
            RtspStatus::Reserved => status_description::RESERVED,
            RtspStatus::NotEnoughBandwidth => status_description::NOT_ENOUGH_BANDWIDTH,
            RtspStatus::SessionNotFound => status_description::SESSION_NOT_FOUND,
            RtspStatus::MethodNotValidInThisState => {
                status_description::METHOD_NOT_VALID_IN_THIS_STATE
            }
            RtspStatus::HeaderFieldNotValidForResource => {
                status_description::HEADER_FIELD_NOT_VALID_FOR_RESOURCE
            }
            RtspStatus::InvalidRange => status_description::INVALID_RANGE,
            RtspStatus::ParameterIsReadOnly => status_description::PARAMETER_IS_READ_ONLY,
            RtspStatus::AggregateOperationNotAllowed => {
                status_description::AGGREGATE_OPERATION_NOT_ALLOWED
            }
            RtspStatus::OnlyAggregateOperationAllowed => {
                status_description::ONLY_AGGREGATE_OPERATION_ALLOWED
            }
            RtspStatus::UnsupportedTransport => status_description::UNSUPPORTED_TRANSPORT,
            RtspStatus::DestinationUnreachable => status_description::DESTINATION_UNREACHABLE,
            RtspStatus::DestinationProhibited => status_description::DESTINATION_PROHIBITED,
            RtspStatus::DataTransportNotReadyYet => {
                status_description::DATA_TRANSPORT_NOT_READY_YET
            }
            RtspStatus::NotificationReasonUnknown => {
                status_description::NOTIFICATION_REASON_UNKNOWN
            }
            RtspStatus::KeyManagementError => status_description::KEY_MANAGEMENT_ERROR,
            RtspStatus::ConnectionAuthorizationRequired => {
                status_description::CONNECTION_AUTHORIZATION_REQUIRED
            }
            RtspStatus::ConnectionCredentialsNotAccepted => {
                status_description::CONNECTION_CREDENTIALS_NOT_ACCEPTED
            }
            RtspStatus::FailureToEstablishSecureConnection => {
                status_description::FAILURE_TO_ESTABLISH_SECURE_CONNECTION
            }
            RtspStatus::InternalServerError => status_description::INTERNAL_SERVER_ERROR,
            RtspStatus::NotImplemented => status_description::NOT_IMPLEMENTED,
            RtspStatus::BadGateWay => status_description::BAD_GATEWAY,
            RtspStatus::ServiceUnavailable => status_description::SERVICE_UNAVAILABLE,
            RtspStatus::GatewayTimeout => status_description::GATEWAY_TIMEOUT,
            RtspStatus::RtspVersionNotSupported => status_description::RTSP_VERSION_NOT_SUPPORTED,
            RtspStatus::OptionNotSupported => status_description::OPTION_NOT_SUPPORTED,
            RtspStatus::ProxyUnavailable => status_description::PROXY_UNAVAILABLE,
        }
    }
}

impl Display for RtspStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let description: &str = (*self).into();
        f.write_str(format!("{} {}", Into::<u16>::into(*self), description).as_str())
    }
}
