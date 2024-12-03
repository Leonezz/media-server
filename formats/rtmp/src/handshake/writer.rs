use std::io;

use super::{C1S1Packet, C2S2Packet, Version, errors::HandshakeResult};
use byteorder::{BigEndian, WriteBytesExt};

pub struct Writer<W> {
    inner: W,
}

impl<W> Writer<W> {
    pub fn into_inner(self) -> W {
        self.inner
    }

    pub fn inner(&self) -> &W {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut W {
        &mut self.inner
    }
}

impl<W> Writer<W>
where
    W: io::Write,
{
    pub fn write_c0s0(&mut self, version: Version) -> HandshakeResult<()> {
        self.inner.write_u8(version.into())?;
        Ok(())
    }

    pub fn write_c1s1(&mut self, packet: C1S1Packet) -> HandshakeResult<()> {
        self.inner
            .write_u32::<BigEndian>(packet.timestamp.as_millis() as u32)?;
        self.inner.write_u32::<BigEndian>(0)?;
        self.inner.write_all(&packet.random_bytes)?;
        Ok(())
    }

    pub fn write_c2s2(&mut self, packet: C2S2Packet) -> HandshakeResult<()> {
        self.inner
            .write_u32::<BigEndian>(packet.timestamp.as_millis() as u32)?;
        self.inner
            .write_u32::<BigEndian>(packet.timestamp2.as_millis() as u32)?;
        self.inner.write_all(&packet.random_echo)?;
        Ok(())
    }
}
