use tokio_util::bytes::BytesMut;

pub mod reader;
pub mod writer;

///! @see: RFC 3550 5.1 RTP Fixed Header Fields
/// this is not likely to useful

///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |V=2|P|X|  CC   |M|      PT     |        sequence number        |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                            timestamp                          |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |            synchronization source (SSRC) identifier           |
/// +=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
/// |             contributing source (CSRC) identifiers            |
/// |                               ....                            |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Debug)]
pub struct RtpHeader {
    version: u8,
    padding: bool,
    extension: bool,
    csrc_count: u8,
    marker: bool,
    payload_type: u8,
    sequence_number: u16,
    timestamp: u32,
    ssrc: u32,
    csrc_list: Vec<u32>,
    header_extension: Option<RtpHeaderExtension>,
}

#[derive(Debug)]
pub struct RtpHeaderExtension {
    profile_defined: u16,
    length: u16,
    bytes: BytesMut,
}
