use std::io::{self};

use h264_codec::nalu::{NalUnit, NaluHeader};
use utils::traits::{reader::ReadRemainingFrom, writer::WriteTo};

use crate::errors::RtpError;

///! @see: RFC 6184 5.6. Single NAL Unit Packet
///  0                    1                  2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |F|NRI|  Type   |                                               |
/// +-+-+-+-+-+-+-+-+                                               |
/// |                                                               |
/// |               Bytes 2..n of a single NAL unit                 |
/// |                                                               |
/// |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                               :...OPTIONAL RTP padding        |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

#[derive(Debug)]
pub struct SingleNalUnit(NalUnit);

impl<R: io::Read> ReadRemainingFrom<u8, R> for SingleNalUnit {
    type Error = RtpError;
    fn read_remaining_from(header: u8, reader: R) -> Result<Self, Self::Error> {
        let nal_header: NaluHeader = header.try_into()?;
        Ok(Self(NalUnit::read_remaining_from(nal_header, reader)?))
    }
}

impl<W: io::Write> WriteTo<W> for SingleNalUnit {
    type Error = RtpError;
    fn write_to(&self, writer: W) -> Result<(), Self::Error> {
        self.0.write_to(writer)?;
        Ok(())
    }
}
