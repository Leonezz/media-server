use url::Url;

use crate::{
    consts::{methods::RtspMethod, version::RtspVersion},
    errors::{RtspMessageError, RtspMessageResult},
    header::{RtspHeader, RtspHeaders},
};

use super::RtspRequest;

#[derive(Debug, Default)]
pub struct RtspRequestBuilder {
    method: Option<RtspMethod>,
    uri: Option<Url>,
    version: Option<RtspVersion>,
    headers: RtspHeaders,
    body: Option<String>,
}

impl RtspRequestBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn method(mut self, method: RtspMethod) -> Self {
        self.method = Some(method);
        self
    }

    pub fn uri(mut self, uri: Url) -> Self {
        self.uri = Some(uri);
        self
    }

    pub fn version(mut self, version: RtspVersion) -> Self {
        self.version = Some(version);
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

    pub fn build(mut self) -> RtspMessageResult<RtspRequest> {
        if self.method.is_none() {
            return Err(RtspMessageError::UnknownMethod(None));
        }

        if self.uri.is_none() {
            return Err(RtspMessageError::UnknownUri(None));
        }

        if !self.uri.as_ref().unwrap().scheme().starts_with("rtsp") {
            return Err(RtspMessageError::UnknownUri(self.uri));
        }

        if let Some(body) = &self.body {
            // TODO: check weather the method allows a body
            if !self.headers.contains(RtspHeader::ContentType) {
                return Err(RtspMessageError::MissingContentType);
            }

            self.headers.remove(RtspHeader::ContentLength);
            self.headers
                .push(RtspHeader::ContentLength, format!("{}", body.len()));
        }

        Ok(RtspRequest {
            method: self.method.unwrap(),
            uri: self.uri.unwrap(),
            version: self.version.unwrap_or_default(),
            headers: self.headers,
            body: self.body,
        })
    }
}
