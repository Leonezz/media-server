use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use h264_codec::nalu::NalUnit;
use std::io;
use utils::traits::{reader::ReadRemainingFrom, writer::WriteTo};

use crate::{codec::h264::util, errors::RtpError};

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

#[derive(Debug)]
pub struct Mtap16Format {
    pub header: u8,
    pub decode_order_number_base: u16,
    pub nal_units: Vec<MtapNalUnit<u16>>,
}

impl<R: io::Read> ReadRemainingFrom<u8, R> for Mtap16Format {
    type Error = RtpError;
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
    type Error = RtpError;
    fn write_to(&self, mut writer: W) -> Result<(), Self::Error> {
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

#[derive(Debug)]
pub struct Mtap24Format {
    pub header: u8,
    pub decode_order_number_base: u16,
    pub nal_units: Vec<MtapNalUnit<u32>>,
}

impl<R: io::Read> ReadRemainingFrom<u8, R> for Mtap24Format {
    type Error = RtpError;
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
    type Error = RtpError;
    fn write_to(&self, mut writer: W) -> Result<(), Self::Error> {
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
