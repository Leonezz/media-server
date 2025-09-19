use builder::AuHeaderBuilder;

pub mod builder;
pub mod packet_size;
pub mod reader;
pub mod writer;
///
/// +---------------------------------------+
/// |             AU-size                   |
/// +---------------------------------------+
/// |       AU-Index / AU-Index-delta       |
/// +---------------------------------------+
/// |            CTS-flag                   |
/// +---------------------------------------+
/// |            CTS-delta                  |
/// +---------------------------------------+
/// |            DTS-flag                   |
/// +---------------------------------------+
/// |            DTS-delta                  |
/// +---------------------------------------+
/// |            RAP-flag                   |
/// +---------------------------------------+
/// |            Stream-state               |
/// +---------------------------------------+
#[derive(Debug, Default, Clone)]
pub struct AuHeader {
    pub bits_cnt: u64,
    pub au_size: Option<u64>,
    pub au_index: Option<u64>,
    pub au_index_delta: Option<u64>,
    pub cts_delta: Option<u64>,
    pub dts_delta: Option<u64>,
    pub rap_flag: Option<bool>,
    pub stream_state: Option<u64>,
}

impl AuHeader {
    pub fn builder() -> AuHeaderBuilder {
        AuHeaderBuilder::new()
    }
}

/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+- .. -+-+-+-+-+-+-+-+-+-+
/// |AU-headers-length|AU-header|AU-header|      |AU-header|padding|
/// |                 |   (1)   |   (2)   |      |   (n)   |  bits |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+- .. -+-+-+-+-+-+-+-+-+-+
#[derive(Debug)]
pub struct AuHeaderSection {
    pub au_headers_length: u64,
    pub au_headers: Vec<AuHeader>,
}
