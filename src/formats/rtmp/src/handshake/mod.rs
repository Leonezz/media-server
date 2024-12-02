use core::time;

pub mod writer;
pub mod reader;
pub mod errors;

#[derive(Debug)]
pub struct C0S0Packet {
    version: Version,
}

pub struct C1S1Packet {
    timestamp: time::Duration,
    random_bytes: [u8; 1528],
}

pub struct C2S2Packet {
    timestamp: time::Duration,
    timestamp2: time::Duration,
    random_echo: [u8; 1528],
}

#[derive(Debug)]
pub enum Version {
    V0 = 0,
    V1 = 1,
    V2 = 2,
    V3 = 3,
}

impl Into<u8> for Version {
    fn into(self) -> u8 {
        match self {
            Version::V0 => 0,
            Version::V1 => 1,
            Version::V2 => 2,
            Version::V3 => 3,
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum HandshakeClientState {
    WriteC0C1,
    ReadS0S1S2,
    WriteC2,
    Finish,
}

#[derive(Clone)]
pub enum HandshakeServerState {
    ReadC0C1,
    WriteS0S1S2,
    ReadC2,
    Finish,
}

pub const RTMP_VERSION: u8 = 3;
