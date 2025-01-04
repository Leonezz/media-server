use crate::errors::RtpError;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtcpPayloadType {
    SenderReport = 200,
    ReceiverReport = 201,
    SourceDescription = 202,
    Bye = 203,
    App = 204,
}

impl TryFrom<u8> for RtcpPayloadType {
    type Error = RtpError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            200 => Ok(Self::SenderReport),
            201 => Ok(Self::ReceiverReport),
            202 => Ok(Self::SourceDescription),
            203 => Ok(Self::Bye),
            204 => Ok(Self::App),
            _ => Err(RtpError::UnknownRtcpPayloadType(value)),
        }
    }
}

impl Into<u8> for RtcpPayloadType {
    fn into(self) -> u8 {
        self as u8
    }
}
