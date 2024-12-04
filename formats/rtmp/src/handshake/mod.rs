use core::time;

pub mod errors;
pub mod reader;
pub mod writer;
pub mod consts;
pub mod codec;

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
    VersionSent,
    S0S1Recived,
    AckSent,
    Done,
}

#[derive(Clone)]
pub enum HandshakeServerState {
    Uninitialized,
    C0Recived,
    VersionSent,
    C1Recived,
    AckSent,
    Done,
}

pub const RTMP_VERSION: u8 = 3;
