use core::time;
use std::io;

use super::{
    C0S0Packet, C1S1Packet, C2S2Packet, Version,
    errors::{HandshakeError, HandshakeResult},
};
use byteorder::{BigEndian, ReadBytesExt};

pub struct Reader<R> {
    inner: R,
}

impl<R> Reader<R> {
    pub fn into_inner(self) -> R {
        self.inner
    }

    pub fn inner(&self) -> &R {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut R {
        &mut self.inner
    }
}

impl<R> Reader<R>
where
    R: io::Read,
{
    pub fn new(inner: R) -> Self {
        Self { inner }
    }

    pub fn read_c0s0(&mut self) -> HandshakeResult<C0S0Packet> {
        let version = self.inner.read_u8()?;
        let version = match version {
            0 => Version::V0,
            1 => Version::V1,
            2 => Version::V2,
            3 => Version::V3,
            _ => return Err(HandshakeError::BadVersion(version)),
        };
        Ok(C0S0Packet { version })
    }

    pub fn read_c1s1(&mut self) -> HandshakeResult<C1S1Packet> {
        let time = self.inner.read_u32::<BigEndian>()?;
        let zero = self.inner.read_u32::<BigEndian>()?;
        let mut buf = [0; 1528];
        self.inner.read_exact(&mut buf)?;
        Ok(C1S1Packet {
            timestamp: time::Duration::from_millis(time as u64),
            _zeros: zero,
            random_bytes: buf,
        })
    }

    pub fn read_c2s2(&mut self) -> HandshakeResult<C2S2Packet> {
        let time = self.inner.read_u32::<BigEndian>()?;
        let time2 = self.inner.read_u32::<BigEndian>()?;
        let mut buf = [0; 1528];
        self.inner.read_exact(&mut buf)?;
        Ok(C2S2Packet {
            timestamp: time::Duration::from_millis(time as u64),
            timestamp2: time::Duration::from_millis(time2 as u64),
            random_echo: buf,
        })
    }
}
