use std::io;

use super::{C0S0Packet, C1S1Packet, C2S2Packet, errors::HandshakeError};
use byteorder::{BigEndian, WriteBytesExt};
use utils::traits::writer::WriteTo;

impl<W: io::Write> WriteTo<W> for C0S0Packet {
    type Error = HandshakeError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u8(self.version.into())?;
        Ok(())
    }
}

impl<W: io::Write> WriteTo<W> for C1S1Packet {
    type Error = HandshakeError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        // timestamp overflows u32
        writer.write_u32::<BigEndian>(self.timestamp.as_millis() as u32)?;
        writer.write_u32::<BigEndian>(0)?;
        writer.write_all(&self.random_bytes)?;
        Ok(())
    }
}

impl<W: io::Write> WriteTo<W> for C2S2Packet {
    type Error = HandshakeError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u32::<BigEndian>(self.timestamp.as_millis() as u32)?;
        writer.write_u32::<BigEndian>(self.timestamp2.as_millis() as u32)?;
        writer.write_all(&self.random_echo)?;
        Ok(())
    }
}
