use std::io;

use crate::errors::FLVResult;

pub mod reader;
pub mod writer;

///
/// 0                   1                   2                   3
/// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |       F       |       L       |       V       |    Version    |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// | Reserved|a|r|v|                  data_offset                  |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |               |
/// +-+-+-+-+-+-+-+-+
#[derive(Debug)]
pub struct FLVHeader {
    flv_marker: [u8; 3], // should always be flv
    flv_version: u8,     // flv version
    // reserved: 5 bits,       // should be 0
    has_audio: bool, // 1 means there are audio tags
    // type_flags_reserved: bits, // should be 0
    has_video: bool,  // 1 means there are video tags
    data_offset: u32, // serves as header bytes length
}

impl FLVHeader {
    pub fn new(has_audio: bool, has_video: bool) -> Self {
        Self {
            flv_marker: [b'F', b'L', b'V'],
            flv_version: 1,
            has_audio: has_audio,
            has_video: has_video,
            data_offset: 9,
        }
    }
    pub fn read_from<R>(reader: R) -> FLVResult<FLVHeader>
    where
        R: io::Read,
    {
        reader::Reader::new(reader).read()
    }

    pub fn write_to<W>(&self, writer: W) -> FLVResult<()>
    where
        W: io::Write,
    {
        writer::Writer::new(writer).write(self)
    }
}
