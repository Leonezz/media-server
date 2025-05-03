use core::time;
use std::io;

use super::{C0S0Packet, C1S1Packet, C2S2Packet, Version, errors::HandshakeError};
use byteorder::{BigEndian, ReadBytesExt};
use utils::traits::reader::ReadFrom;

impl<R: io::Read> ReadFrom<R> for C0S0Packet {
    type Error = HandshakeError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let version = reader.read_u8()?;
        let version = match version {
            0 => Version::V0,
            1 => Version::V1,
            2 => Version::V2,
            3 => Version::V3,
            _ => return Err(HandshakeError::BadVersion(version)),
        };
        Ok(C0S0Packet { version })
    }
}

impl<R: io::Read> ReadFrom<R> for C1S1Packet {
    type Error = HandshakeError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let time = reader.read_u32::<BigEndian>()?;
        let zero = reader.read_u32::<BigEndian>()?;
        let mut buf = [0; 1528];
        reader.read_exact(&mut buf)?;
        Ok(C1S1Packet {
            timestamp: time::Duration::from_millis(time as u64),
            _zeros: zero,
            random_bytes: buf,
        })
    }
}

impl<R: io::Read> ReadFrom<R> for C2S2Packet {
    type Error = HandshakeError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let time = reader.read_u32::<BigEndian>()?;
        let time2 = reader.read_u32::<BigEndian>()?;
        let mut buf = [0; 1528];
        reader.read_exact(&mut buf)?;
        Ok(C2S2Packet {
            timestamp: time::Duration::from_millis(time as u64),
            timestamp2: time::Duration::from_millis(time2 as u64),
            random_echo: buf,
        })
    }
}
