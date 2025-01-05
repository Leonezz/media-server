use crate::errors::RtpError;

///! @see: RFC 3551 6. Payload Type Definitions
#[repr(u8)]
#[derive(Debug)]
pub enum AvpAudioPayloadType {
    PCMU = 0,
    GSM = 3,
    G723 = 4,
    DVI4_8000 = 5,
    DVI4_16000 = 6,
    LPC = 7,
    PCMA = 8,
    G722 = 9,
    L16Channel2 = 10,
    L16Channel1 = 11,
    QCELP = 12,
    CN = 13,
    MPA = 14,
    G728 = 15,
    DVI4_11025 = 16,
    DVI4_22050 = 17,
    G729 = 18,
    Reserved(u8),   // 1, 2, 19
    Unassigned(u8), // 20 - 23
}

impl Into<u8> for AvpAudioPayloadType {
    fn into(self) -> u8 {
        match self {
            Self::Reserved(v) | Self::Unassigned(v) => v,
            Self::PCMU => 0,
            Self::GSM => 3,
            Self::G723 => 4,
            Self::DVI4_8000 => 5,
            Self::DVI4_16000 => 6,
            Self::LPC => 7,
            Self::PCMA => 8,
            Self::G722 => 9,
            Self::L16Channel2 => 10,
            Self::L16Channel1 => 11,
            Self::QCELP => 12,
            Self::CN => 13,
            Self::MPA => 14,
            Self::G728 => 15,
            Self::DVI4_11025 => 16,
            Self::DVI4_22050 => 17,
            Self::G729 => 18,
        }
    }
}

impl TryFrom<u8> for AvpAudioPayloadType {
    type Error = RtpError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            v if v == 1 || v == 2 || v == 19 => Ok(Self::Reserved(v)),
            v if v >= 20 && v <= 24 => Ok(Self::Unassigned(v)),
            0 => Ok(Self::PCMU),
            3 => Ok(Self::GSM),
            4 => Ok(Self::G723),
            5 => Ok(Self::DVI4_8000),
            6 => Ok(Self::DVI4_16000),
            7 => Ok(Self::LPC),
            8 => Ok(Self::PCMA),
            9 => Ok(Self::G722),
            10 => Ok(Self::L16Channel2),
            11 => Ok(Self::L16Channel1),
            12 => Ok(Self::QCELP),
            13 => Ok(Self::CN),
            14 => Ok(Self::MPA),
            15 => Ok(Self::G728),
            16 => Ok(Self::DVI4_11025),
            17 => Ok(Self::DVI4_22050),
            18 => Ok(Self::G729),
            _ => Err(RtpError::WrongPayloadType(format!(
                "expect audio payload type of avp profile, got unknown {}",
                value
            ))),
        }
    }
}

#[repr(u8)]
#[derive(Debug)]
pub enum AvpVideoPayloadType {
    Unassigned(u8), // 24, 27, 29, 30
    CelB = 25,
    JPEG = 26,
    NV = 28,
    H261 = 31,
    MPV = 32,
    H263 = 34,
}

impl Into<u8> for AvpVideoPayloadType {
    fn into(self) -> u8 {
        match self {
            Self::Unassigned(v) => v,
            Self::CelB => 25,
            Self::JPEG => 26,
            Self::NV => 28,
            Self::H261 => 31,
            Self::MPV => 32,
            Self::H263 => 34,
        }
    }
}

impl TryFrom<u8> for AvpVideoPayloadType {
    type Error = RtpError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            v if [24 as u8, 27, 29, 30].contains(&v) => Ok(Self::Unassigned(v)),
            25 => Ok(Self::CelB),
            26 => Ok(Self::JPEG),
            28 => Ok(Self::NV),
            31 => Ok(Self::H261),
            32 => Ok(Self::MPV),
            34 => Ok(Self::H263),
            _ => Err(RtpError::WrongPayloadType(format!(
                "expect video payload type of avp profile got unknown {}",
                value
            ))),
        }
    }
}

#[repr(u8)]
#[derive(Debug)]
pub enum AvpPayloadType {
    Audio(AvpAudioPayloadType),
    Video(AvpVideoPayloadType),
    MP2T = 33, // AV
    Reserved(u8),
    Unassigned(u8),
    Dynamic(u8),
}

impl Into<u8> for AvpPayloadType {
    fn into(self) -> u8 {
        match self {
            Self::Audio(pt) => pt.into(),
            Self::Video(pt) => pt.into(),
            Self::MP2T => 33,
            Self::Reserved(v) | Self::Unassigned(v) | Self::Dynamic(v) => v,
        }
    }
}

impl From<u8> for AvpPayloadType {
    fn from(value: u8) -> Self {
        if (value >= 35 && value <= 71) || (value >= 77 && value <= 95) {
            return Self::Unassigned(value);
        }

        if value >= 72 && value <= 76 {
            return Self::Reserved(value);
        }

        if value >= 96 && value <= 127 {
            return Self::Dynamic(value);
        }

        if value == 33 {
            return Self::MP2T;
        }

        if let Ok(audio) = AvpAudioPayloadType::try_from(value) {
            return Self::Audio(audio);
        }

        if let Ok(video) = AvpVideoPayloadType::try_from(value) {
            return Self::Video(video);
        }

        return Self::Dynamic(value);
    }
}
