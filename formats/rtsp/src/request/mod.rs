pub mod builder;
pub mod framed;
pub mod reader;
#[cfg(test)]
mod test;
use std::fmt;

use url::Url;

use crate::{
    consts::{common::CRLF_STR, methods::RtspMethod, version::RtspVersion},
    header::RtspHeaders,
};

use builder::RtspRequestBuilder;

#[derive(Debug, Clone)]
pub struct RtspRequest {
    pub(crate) method: RtspMethod,
    pub(crate) uri: Url,
    pub(crate) version: RtspVersion,
    pub(crate) headers: RtspHeaders,
    pub(crate) body: Option<String>,
}

impl RtspRequest {
    pub fn builder() -> RtspRequestBuilder {
        RtspRequestBuilder::new()
    }

    pub fn method(&self) -> RtspMethod {
        self.method
    }

    pub fn uri(&self) -> &Url {
        &self.uri
    }

    pub fn version(&self) -> &RtspVersion {
        &self.version
    }

    pub fn headers(&self) -> &RtspHeaders {
        &self.headers
    }

    pub fn body(&self) -> Option<&String> {
        self.body.as_ref()
    }
}

impl fmt::Display for RtspRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {}{}",
            self.method, self.uri, self.version, CRLF_STR
        )?;
        write!(f, "{}{}", self.headers, CRLF_STR)?;
        if let Some(body) = &self.body {
            f.write_str(body)?;
        }
        Ok(())
    }
}
