use std::io;

use crate::errors::CodecCommonError;
use byteorder::{BigEndian, WriteBytesExt};
use num::ToPrimitive;
use utils::traits::dynamic_sized_packet::DynamicSizedPacket;
use utils::traits::writer::WriteTo;

use super::VideoFrameUnit;

impl<W: io::Write> WriteTo<W> for VideoFrameUnit {
    type Error = CodecCommonError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        match self {
            Self::H264 { nal_units } => nal_units.iter().try_for_each(|unit| {
                writer.write_all(&[0, 0, 1])?;
                unit.write_to(writer)?;
                Ok::<(), Self::Error>(())
            })?,
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct VideoFrameUnitAvccWriter<'a>(pub &'a VideoFrameUnit, pub u8);

impl<'a, W: io::Write> WriteTo<W> for VideoFrameUnitAvccWriter<'a> {
    type Error = CodecCommonError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let (frame_unit, nalu_size_length) = (self.0, self.1);
        if nalu_size_length != 1 && nalu_size_length != 2 && nalu_size_length != 4 {
            return Err(CodecCommonError::InvalidNaluSizeLengthMinueOne(
                nalu_size_length - 1,
            ));
        }
        match frame_unit {
            VideoFrameUnit::H264 { nal_units } => nal_units.iter().try_for_each(|unit| {
                let nalu_size = unit.get_packet_bytes_count();
                assert!(nalu_size > 0);
                match nalu_size_length {
                    1 => {
                        assert!(nalu_size <= u8::MAX.to_usize().unwrap());
                        writer.write_u8(nalu_size.to_u8().unwrap())?
                    }
                    2 => {
                        assert!(nalu_size <= u16::MAX.to_usize().unwrap());
                        writer.write_u16::<BigEndian>(nalu_size.to_u16().unwrap())?
                    }
                    4 => {
                        assert!(nalu_size <= u32::MAX.to_usize().unwrap());
                        writer.write_u32::<BigEndian>(nalu_size.to_u32().unwrap())?
                    }
                    _ => unreachable!(),
                }
                unit.write_to(writer)?;
                Ok::<(), Self::Error>(())
            })?,
        }
        Ok(())
    }
}
