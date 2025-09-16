use super::au_header::AuHeader;
use std::fmt;
use tokio_util::{
    bytes::{Bytes, BytesMut},
    either::Either,
};
pub mod packet_size;
pub mod reader;
pub mod writer;
pub struct AccessUnit {
    pub header: AuHeader,
    pub body: Bytes,

    pub presentation_timestamp_ms: u32,
}

impl fmt::Debug for AccessUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "au_header: {:?}, body len: {}, timestamp: {}",
            self.header,
            self.body.len(),
            self.presentation_timestamp_ms
        )
    }
}

#[derive(Debug)]
pub struct AccessUnitFragment {
    pub header: AuHeader,
    pub body: BytesMut,

    pub timestamp: u32,
}
impl AccessUnitFragment {
    pub fn complete(self) -> AccessUnit {
        AccessUnit {
            header: self.header,
            body: self.body.freeze(),

            presentation_timestamp_ms: self.timestamp,
        }
    }
}

/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |AU(1)                                                          |
/// +                                                               |
/// |                                                               |
/// |               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |               |AU(2)                                          |
/// +-+-+-+-+-+-+-+-+                                               |
/// |                                                               |
/// |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                               | AU(n)                         |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |AU(n) continued|
/// |-+-+-+-+-+-+-+-+
#[derive(Debug)]
pub struct AccessUnitSection {
    pub access_units_or_fragment: Either<Vec<AccessUnit>, AccessUnitFragment>,
}
