use crate::{
    consts::{headers::RtspHeader, status::RtspStatus, version::RtspVersion},
    errors::{RTSPMessageError, RTSPResult},
    header::RtspHeaders,
};

use super::RtspResponse;

#[derive(Debug, Default)]
pub struct RtspResponseBuilder {
    pub(crate) version: Option<RtspVersion>,
    pub(crate) status: Option<RtspStatus>,
    pub(crate) headers: RtspHeaders,
    pub(crate) body: Option<String>,
}

impl RtspResponseBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn version(mut self, version: RtspVersion) -> Self {
        self.version = Some(version);
        self
    }

    pub fn status(mut self, status: RtspStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn header<S: Into<String>>(mut self, key: RtspHeader, value: S) -> Self {
        self.headers.push(key, value.into());
        self
    }

    pub fn headers(mut self, headers: Vec<(RtspHeader, String)>) -> Self {
        self.headers.append(headers);
        self
    }

    pub fn body(mut self, body: String) -> Self {
        self.body = Some(body);
        self
    }

    pub fn build(mut self) -> RTSPResult<RtspResponse> {
        if self.status.is_none() {
            return Err(RTSPMessageError::UnknownStatusCode(None));
        }

        if let Some(body) = &self.body {
            // TODO: check weather the method allows a body
            if !self.headers.contains(RtspHeader::ContentType) {
                return Err(RTSPMessageError::MissingContentType);
            }

            self.headers.remove(RtspHeader::ContentLength);
            self.headers
                .push(RtspHeader::ContentLength, format!("{}", body.len()));
        }

        Ok(RtspResponse {
            version: self.version.unwrap_or(RtspVersion::V2),
            status: self.status.unwrap(),
            headers: self.headers,
            body: self.body,
        })
    }
}
