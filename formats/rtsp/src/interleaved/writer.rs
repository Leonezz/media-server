use byteorder::{BigEndian, WriteBytesExt};
use num::ToPrimitive;
use std::io;
use utils::traits::writer::WriteTo;

use crate::errors::RtspMessageError;

use super::{DOLLAR_SIGN, RtspInterleavedPacket};

impl<W: io::Write> WriteTo<W> for RtspInterleavedPacket {
    type Error = RtspMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u8(DOLLAR_SIGN)?;
        writer.write_u8(self.channel_id)?;
        writer.write_u16::<BigEndian>(self.payload.len().to_u16().ok_or(
            RtspMessageError::InvalidInterleavedDataLength(self.payload.len()),
        )?)?;
        writer.write_all(&self.payload)?;
        Ok(())
    }
}
