use core::time;

pub mod consts;
pub mod digest;
pub mod errors;
pub mod reader;
pub mod server;
pub mod writer;

#[derive(Debug)]
pub struct C0S0Packet {
    version: Version,
}

pub struct C1S1Packet {
    timestamp: time::Duration,
    _zeros: u32,
    random_bytes: [u8; 1528],
}

pub struct C2S2Packet {
    timestamp: time::Duration,
    timestamp2: time::Duration,
    random_echo: [u8; 1528],
}

#[derive(Debug, Clone, Copy)]
pub enum Version {
    V0 = 0,
    V1 = 1,
    V2 = 2,
    V3 = 3,
}

impl From<Version> for u8 {
    fn from(value: Version) -> Self {
        match value {
            Version::V0 => 0,
            Version::V1 => 1,
            Version::V2 => 2,
            Version::V3 => 3,
        }
    }
}

/// +-------------+                +-------------+
/// |    Client   | TCP/IP Network |    Server   |
/// +-------------+       |        +-------------+
///        |              |               |
///  Uninitialized        |         Uninitialized
///        |      C0      |               |
///        |------------->|        C0     |
///        |              |-------------->|
///        |      C1      |               |
///        |------------->|        S0     |
///        |              |<--------------|
///        |              |        S1     |
///  Version sent         |<--------------|
///        |      S0      |               |
///        |<-------------|               |
///        |      S1      |               |
///        |<-------------|         Version sent
///        |              |        C1     |
///        |              |-------------->|
///        |      C2      |               |
///        |------------->|        S2     |
///        |              |<--------------|
///     Ack sent          |            Ack Sent
///        |      S2      |               |
///        |<-------------|               |
///        |              |        C2     |
///        |              |-------------->|
///   Handshake Done      |          Handshake Done
///        |              |               |
///     Pictorial Representation of Handshake
#[derive(PartialEq, Eq)]
pub enum HandshakeClientState {
    Uninitialized,
    C0C1Rent,
    S0S1Recived,
    AckSent,
    Done,
}

#[derive(Debug, Clone)]
pub enum HandshakeServerState {
    Uninitialized,
    C0C1Recived,
    S0S1S2Sent,
    Done,
}

pub const RTMP_VERSION: Version = Version::V3;
