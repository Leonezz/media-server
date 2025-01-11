use std::io::{self};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use h264_codec::nalu::NalUnit;
use utils::traits::{reader::ReadRemainingFrom, writer::WriteTo};

use crate::{codec::h264::util, errors::RtpError};

///! @see: RFC 6184 5.7.1. Single-Time Aggregation Packet (STAP) Figure 7
///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                         RTP Header                            |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |STAP-A NAL HDR |        NALU 1 Size            |   NALU 1 HDR  |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                        NALU 1 Data                            |
/// :                                                               :
/// +               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |               |            NALU 2 Size        |  NALU 2 HDR   |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                        NALU 2 Data                            |
/// :                                                               :
/// |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                               :...OPTIONAL RTP padding        |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

#[derive(Debug)]
pub struct StapAPacket {
    pub header: u8,
    pub nal_units: Vec<NalUnit>,
}

impl<R: io::Read> ReadRemainingFrom<u8, R> for StapAPacket {
    type Error = RtpError;
    fn read_remaining_from(header: u8, reader: R) -> Result<Self, Self::Error> {
        let nal_units = util::read_aggregated_trivial_nal_units(reader)?;

        Ok(Self { header, nal_units })
    }
}

impl<W: io::Write> WriteTo<W> for StapAPacket {
    type Error = RtpError;
    fn write_to(&self, mut writer: W) -> Result<(), Self::Error> {
        writer.write_u8(self.header)?;

        self.nal_units
            .iter()
            .try_for_each(|nalu| util::write_aggregated_stap_nal_unit(writer.by_ref(), nalu))?;

        Ok(())
    }
}

///! @see: Figure 8
///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                         RTP Header                            |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |STAP-B NAL HDR |            DON                |  NALU 1 Size  |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |  NALU 1 Size  |  NALU 1 HDR   |          NALU 1 Data          |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+                               +
/// :                                                               :
/// +               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |               |           NALU 2 Size         |  NALU 2 HDR   |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                       NALU 2 Data                             |
/// :                                                               :
/// |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                               :...OPTIONAL RTP padding        |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Debug)]
pub struct StapBPacket {
    pub header: u8,
    pub decode_order_number: u16,
    pub nal_units: Vec<NalUnit>,
}

impl<R: io::Read> ReadRemainingFrom<u8, R> for StapBPacket {
    type Error = RtpError;
    fn read_remaining_from(header: u8, mut reader: R) -> Result<Self, Self::Error> {
        let decode_order_number = reader.read_u16::<BigEndian>()?;
        let nal_units = util::read_aggregated_trivial_nal_units(reader)?;
        Ok(Self {
            header,
            decode_order_number,
            nal_units,
        })
    }
}

impl<W: io::Write> WriteTo<W> for StapBPacket {
    type Error = RtpError;
    fn write_to(&self, mut writer: W) -> Result<(), Self::Error> {
        writer.write_u8(self.header)?;
        writer.write_u16::<BigEndian>(self.decode_order_number)?;

        self.nal_units
            .iter()
            .try_for_each(|nalu| util::write_aggregated_stap_nal_unit(writer.by_ref(), nalu))?;

        Ok(())
    }
}
