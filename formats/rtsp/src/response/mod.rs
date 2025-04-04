pub mod builder;
pub mod framed;
pub mod reader;
#[cfg(test)]
mod test;
use std::fmt;

use builder::RtspResponseBuilder;

use crate::{
    consts::{common::CRLF_STR, status::RtspStatus, version::RtspVersion},
    header::RtspHeaders,
};

#[derive(Debug, Clone)]
pub struct RtspResponse {
    status: RtspStatus,
    version: RtspVersion,
    headers: RtspHeaders,
    body: Option<String>,
}

impl RtspResponse {
    pub fn builder() -> RtspResponseBuilder {
        RtspResponseBuilder::new()
    }

    pub fn status(&self) -> RtspStatus {
        self.status
    }

    pub fn version(&self) -> &RtspVersion {
        &self.version
    }

    pub fn set_version(&mut self, version: RtspVersion) {
        self.version = version;
    }

    pub fn headers(&self) -> &RtspHeaders {
        &self.headers
    }

    pub fn headers_mut(&mut self) -> &mut RtspHeaders {
        &mut self.headers
    }

    pub fn body(&self) -> &Option<String> {
        &self.body
    }
}

impl fmt::Display for RtspResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}{}", self.version, self.status, CRLF_STR)?;
        write!(f, "{}{}", self.headers, CRLF_STR)?;
        if let Some(body) = &self.body {
            f.write_str(body)?;
        }
        Ok(())
    }
}
