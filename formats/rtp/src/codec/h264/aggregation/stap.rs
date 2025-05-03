use std::io::{self};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use codec_h264::nalu::NalUnit;
use utils::traits::{
    dynamic_sized_packet::DynamicSizedPacket, reader::ReadRemainingFrom, writer::WriteTo,
};

use crate::codec::h264::{errors::RtpH264Error, util};

// @see: RFC 6184 5.7.1. Single-Time Aggregation Packet (STAP) Figure 7
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
pub struct StapAFormat {
    pub header: u8,
    pub nal_units: Vec<NalUnit>,
}

impl<R: io::Read> ReadRemainingFrom<u8, R> for StapAFormat {
    type Error = RtpH264Error;
    fn read_remaining_from(header: u8, reader: R) -> Result<Self, Self::Error> {
        let nal_units = util::read_aggregated_trivial_nal_units(reader)?;

        Ok(Self { header, nal_units })
    }
}

impl<W: io::Write> WriteTo<W> for StapAFormat {
    type Error = RtpH264Error;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u8(self.header)?;

        self.nal_units
            .iter()
            .try_for_each(|nalu| util::write_aggregated_stap_nal_unit(writer.by_ref(), nalu))?;

        Ok(())
    }
}

impl DynamicSizedPacket for StapAFormat {
    fn get_packet_bytes_count(&self) -> usize {
        1 // STAP-A NAL HDR
        + self.nal_units.iter().fold(
            0,
            |prev, cur| 
                prev
                    + 2 // nalu size
                    + cur.get_packet_bytes_count()
        )
    }
}

// @see: Figure 8
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
pub struct StapBFormat {
    pub header: u8,
    pub decode_order_number: u16,
    pub nal_units: Vec<NalUnit>,
}

impl<R: io::Read> ReadRemainingFrom<u8, R> for StapBFormat {
    type Error = RtpH264Error;
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

impl<W: io::Write> WriteTo<W> for StapBFormat {
    type Error = RtpH264Error;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u8(self.header)?;
        writer.write_u16::<BigEndian>(self.decode_order_number)?;

        self.nal_units
            .iter()
            .try_for_each(|nalu| util::write_aggregated_stap_nal_unit(writer.by_ref(), nalu))?;

        Ok(())
    }
}

impl DynamicSizedPacket for StapBFormat {
    fn get_packet_bytes_count(&self) -> usize {
        1 // STAP-B NAL HDR
        + 2 // don
        + self.nal_units.iter().fold(0, |prev, cur| 
            prev 
            + 2 // nalu size
            + cur.get_packet_bytes_count()
        )
    }
}
