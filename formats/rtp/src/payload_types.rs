use crate::rtcp::payload_types::RtcpPayloadType;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadType {
    Rtcp(RtcpPayloadType),
    Unspecified(u8),
}
