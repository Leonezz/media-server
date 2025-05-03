//! @see: RFC 6184 5.7.2. Multi-Time Aggregation Packets (MTAPs)
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use codec_h264::nalu::NalUnit;
use std::io;
use utils::traits::{
    dynamic_sized_packet::DynamicSizedPacket, reader::ReadRemainingFrom, writer::WriteTo,
};

use crate::codec::h264::{errors::RtpH264Error, util};

#[derive(Debug)]
pub struct MtapNalUnit<T: Into<u32>> {
    pub nal_unit: NalUnit,
    pub timestamp_offset: T,
    pub decode_order_number_diff: u8,
}

impl<T: Into<u32>> From<(NalUnit, u8, T)> for MtapNalUnit<T> {
    fn from((nal_unit, decode_order_number_diff, timestamp_offset): (NalUnit, u8, T)) -> Self {
        Self {
            nal_unit,
            timestamp_offset,
            decode_order_number_diff,
        }
    }
}

// @see: Figure 12. An RTP packet including a multi-time aggregation packet of type MTAP16 containing two multi-time aggregation units
///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                         RTP Header                            |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |MTAP16 NAL HDR |  decoding order number base   |  NALU 1 Size  |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |  NALU 1 Size  |  NALU 1 DOND  |        NALU 1 TS offset       |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |   NALU 1 HDR  |                  NALU 1 DATA                  |
/// +-+-+-+-+-+-+-+-+                                               +
/// :                                                               :
/// +               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |               |           NALU 2 SIZE         |  NALU 2 DOND  |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// | NALU 2 TS offset              |  NALU 2 HDR   |  NALU 2 DATA  |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+               |
/// :                                                               :
/// |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                               :...OPTIONAL RTP padding        |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Debug)]
pub struct Mtap16Format {
    pub header: u8,
    pub decode_order_number_base: u16,
    pub nal_units: Vec<MtapNalUnit<u16>>,
}

impl<R: io::Read> ReadRemainingFrom<u8, R> for Mtap16Format {
    type Error = RtpH264Error;
    fn read_remaining_from(header: u8, mut reader: R) -> Result<Self, Self::Error> {
        let decode_order_number_base = reader.read_u16::<BigEndian>()?;
        let nal_units = util::read_aggregated_mtap16_nal_units(reader)?
            .into_iter()
            .map(
                |(nal_unit, decode_order_number_diff, timestamp_offset)| MtapNalUnit {
                    nal_unit,
                    timestamp_offset,
                    decode_order_number_diff,
                },
            )
            .collect();

        Ok(Self {
            header,
            decode_order_number_base,
            nal_units,
        })
    }
}

impl<W: io::Write> WriteTo<W> for Mtap16Format {
    type Error = RtpH264Error;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u8(self.header)?;
        writer.write_u16::<BigEndian>(self.decode_order_number_base)?;
        self.nal_units.iter().try_for_each(|nalu| {
            util::write_aggregated_mtap16_nal_unit(
                writer.by_ref(),
                &nalu.nal_unit,
                nalu.decode_order_number_diff,
                nalu.timestamp_offset,
            )
        })?;
        Ok(())
    }
}

impl DynamicSizedPacket for Mtap16Format {
    fn get_packet_bytes_count(&self) -> usize {
        1 // MTAP16 NAL HDR
        + 2 // donb 
        + self.nal_units.iter().fold(0, |prev, cur| {
            prev
            + 2 // nalu size
            + 1 // dond
            + 2 // timestamp offset
            + cur.nal_unit.get_packet_bytes_count()
        })
    }
}

// @see: Figure 13. An RTP packet including a multi-time aggregation packet of type MTAP24 containing two multi-time aggregation units
///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                         RTP Header                            |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |MTAP24 NAL HDR |  decoding order number base   |  NALU 1 Size  |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |  NALU 1 Size  |  NALU 1 DOND  |        NALU 1 TS offset       |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |NALU 1 TS offs |   NALU 1 HDR  |           NALU 1 DATA         |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+                               +
/// :                                                               :
/// +               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |               |           NALU 2 SIZE         |  NALU 2 DOND  |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// | NALU 2 TS offset                              |   NALU 2 HDR  |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// | NALU 2 DATA                                                   |
/// :                                                               :
/// |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                               :...OPTIONAL RTP padding        |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Debug)]
pub struct Mtap24Format {
    pub header: u8,
    pub decode_order_number_base: u16,
    pub nal_units: Vec<MtapNalUnit<u32>>,
}

impl<R: io::Read> ReadRemainingFrom<u8, R> for Mtap24Format {
    type Error = RtpH264Error;
    fn read_remaining_from(header: u8, mut reader: R) -> Result<Self, Self::Error> {
        let decode_order_number_base = reader.read_u16::<BigEndian>()?;
        let nal_units = util::read_aggregated_mtap24_nal_units(reader)?
            .into_iter()
            .map(
                |(nal_unit, decode_order_number_diff, timestamp_offset)| MtapNalUnit {
                    nal_unit,
                    timestamp_offset,
                    decode_order_number_diff,
                },
            )
            .collect();
        Ok(Self {
            header,
            decode_order_number_base,
            nal_units,
        })
    }
}

impl<W: io::Write> WriteTo<W> for Mtap24Format {
    type Error = RtpH264Error;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u8(self.header)?;
        writer.write_u16::<BigEndian>(self.decode_order_number_base)?;

        self.nal_units.iter().try_for_each(|nalu| {
            util::write_aggregated_mtap24_nal_unit(
                writer.by_ref(),
                &nalu.nal_unit,
                nalu.decode_order_number_diff,
                nalu.timestamp_offset,
            )
        })?;

        Ok(())
    }
}

impl DynamicSizedPacket for Mtap24Format {
    fn get_packet_bytes_count(&self) -> usize {
        1 // MTAP24 NAL HDR 
        + 2 // donb
        + self.nal_units.iter().fold(0, |prev, cur|
            prev
            + 2 // nalu size
            + 1 // dond
            + 3 // timestamp offset
            + cur.nal_unit.get_packet_bytes_count()
        )
    }
}
