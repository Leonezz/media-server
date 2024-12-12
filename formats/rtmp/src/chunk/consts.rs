pub const MAX_TIMESTAMP: u32 = 0xFFFFFF;

pub mod csid {
    use crate::{
        protocol_control::consts::PROTOCOL_CONTROL_CSID, user_control::consts::USER_CONTROL_CSID,
    };

    pub const PROTOCOL_CONTROL: u8 = PROTOCOL_CONTROL_CSID;
    pub const USER_CONTROL: u8 = USER_CONTROL_CSID;
    pub const NET_CONNECTION_COMMAND: u8 = 0x03;
    pub const NET_CONNECTION_COMMAND2: u8 = 0x04;
    pub const NET_STREAM_COMMAND: u8 = 0x05;
    pub const NET_STREAM_COMMAND2: u8 = 0x08;
    pub const AUDIO: u8 = 0x07;
    pub const VIDEO: u8 = 0x06;
}
