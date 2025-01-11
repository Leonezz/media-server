use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use h264_codec::nalu::NalUnit;
use std::io::{self, Cursor, Read};
use tokio_util::bytes::Buf;
use utils::traits::reader::ReadExactFrom;
use utils::traits::writer::WriteTo;

use crate::errors::RtpResult;

fn read_aggregated_nal_units<R: io::Read, Res, F: Fn(&mut Cursor<&Vec<u8>>) -> RtpResult<Res>>(
    mut reader: R,
    func: F,
) -> RtpResult<Vec<Res>> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    let mut cursor = Cursor::new(&bytes);
    let mut result = Vec::new();
    while cursor.has_remaining() {
        let res = func(cursor.by_ref())?;
        result.push(res);
    }
    Ok(result)
}

pub fn read_aggregated_trivial_nal_units<R: io::Read>(reader: R) -> RtpResult<Vec<NalUnit>> {
    let nal_reader = |reader: &mut Cursor<&Vec<u8>>| -> RtpResult<NalUnit> {
        let nal_size = reader.read_u16::<BigEndian>()?;
        let nal_unit = NalUnit::read_exact_from(nal_size as usize, reader)?;
        Ok(nal_unit)
    };

    return read_aggregated_nal_units(reader, nal_reader);
}

pub fn read_aggregated_mtap16_nal_units<R: io::Read>(
    reader: R,
) -> RtpResult<Vec<(NalUnit, u8, u16)>> {
    let nal_reader = |reader: &mut Cursor<&Vec<u8>>| -> RtpResult<(NalUnit, u8, u16)> {
        let nal_size = reader.read_u16::<BigEndian>()?;
        let decode_order_number_diff = reader.read_u8()?;
        let timestamp_offset = reader.read_u16::<BigEndian>()?;
        let nal_unit = NalUnit::read_exact_from((nal_size - 3) as usize, reader)?;
        Ok((nal_unit, decode_order_number_diff, timestamp_offset))
    };

    return read_aggregated_nal_units(reader, nal_reader);
}

pub fn read_aggregated_mtap24_nal_units<R: io::Read>(
    reader: R,
) -> RtpResult<Vec<(NalUnit, u8, u32)>> {
    let nal_reader = |reader: &mut Cursor<&Vec<u8>>| -> RtpResult<(NalUnit, u8, u32)> {
        let nal_size = reader.read_u16::<BigEndian>()?;
        let decode_order_number_diff = reader.read_u8()?;
        let timestamp_offset = reader.read_u24::<BigEndian>()?;
        let nal_unit = NalUnit::read_exact_from((nal_size - 4) as usize, reader)?;
        Ok((nal_unit, decode_order_number_diff, timestamp_offset))
    };

    return read_aggregated_nal_units(reader, nal_reader);
}

pub fn write_aggregated_stap_nal_unit<W: io::Write>(
    mut writer: W,
    nal_unit: &NalUnit,
) -> RtpResult<()> {
    writer.write_u16::<BigEndian>(nal_unit.body.len() as u16 + 1)?;
    nal_unit.write_to(writer)?;
    Ok(())
}

pub fn write_aggregated_mtap16_nal_unit<W: io::Write>(
    mut writer: W,
    nal_unit: &NalUnit,
    decode_order_number_diff: u8,
    timestamp_offset: u16,
) -> RtpResult<()> {
    writer.write_u16::<BigEndian>(nal_unit.body.len() as u16 + 1 + 1 + 2)?;
    writer.write_u8(decode_order_number_diff)?;
    writer.write_u16::<BigEndian>(timestamp_offset)?;
    nal_unit.write_to(writer)?;
    Ok(())
}

pub fn write_aggregated_mtap24_nal_unit<W: io::Write>(
    mut writer: W,
    nal_unit: &NalUnit,
    decode_order_number_diff: u8,
    timestamp_offset: u32,
) -> RtpResult<()> {
    writer.write_u16::<BigEndian>(nal_unit.body.len() as u16 + 1 + 1 + 3)?;
    writer.write_u8(decode_order_number_diff)?;
    writer.write_u24::<BigEndian>(timestamp_offset)?;
    nal_unit.write_to(writer)?;
    Ok(())
}
