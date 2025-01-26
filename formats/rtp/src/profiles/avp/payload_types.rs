use crate::errors::RtpError;

// @see: RFC 3551 6. Payload Type Definitions
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

impl From<AvpAudioPayloadType> for u8 {
    fn from(value: AvpAudioPayloadType) -> Self {
        match value {
            AvpAudioPayloadType::Reserved(v) | AvpAudioPayloadType::Unassigned(v) => v,
            AvpAudioPayloadType::PCMU => 0,
            AvpAudioPayloadType::GSM => 3,
            AvpAudioPayloadType::G723 => 4,
            AvpAudioPayloadType::DVI4_8000 => 5,
            AvpAudioPayloadType::DVI4_16000 => 6,
            AvpAudioPayloadType::LPC => 7,
            AvpAudioPayloadType::PCMA => 8,
            AvpAudioPayloadType::G722 => 9,
            AvpAudioPayloadType::L16Channel2 => 10,
            AvpAudioPayloadType::L16Channel1 => 11,
            AvpAudioPayloadType::QCELP => 12,
            AvpAudioPayloadType::CN => 13,
            AvpAudioPayloadType::MPA => 14,
            AvpAudioPayloadType::G728 => 15,
            AvpAudioPayloadType::DVI4_11025 => 16,
            AvpAudioPayloadType::DVI4_22050 => 17,
            AvpAudioPayloadType::G729 => 18,
        }
    }
}

impl TryFrom<u8> for AvpAudioPayloadType {
    type Error = RtpError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            v if v == 1 || v == 2 || v == 19 => Ok(Self::Reserved(v)),
            v if (20..=24).contains(&v) => Ok(Self::Unassigned(v)),
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

impl From<AvpVideoPayloadType> for u8 {
    fn from(value: AvpVideoPayloadType) -> Self {
        match value {
            AvpVideoPayloadType::Unassigned(v) => v,
            AvpVideoPayloadType::CelB => 25,
            AvpVideoPayloadType::JPEG => 26,
            AvpVideoPayloadType::NV => 28,
            AvpVideoPayloadType::H261 => 31,
            AvpVideoPayloadType::MPV => 32,
            AvpVideoPayloadType::H263 => 34,
        }
    }
}

impl TryFrom<u8> for AvpVideoPayloadType {
    type Error = RtpError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            v if [24_u8, 27, 29, 30].contains(&v) => Ok(Self::Unassigned(v)),
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

impl From<AvpPayloadType> for u8 {
    fn from(value: AvpPayloadType) -> Self {
        match value {
            AvpPayloadType::Audio(pt) => pt.into(),
            AvpPayloadType::Video(pt) => pt.into(),
            AvpPayloadType::MP2T => 33,
            AvpPayloadType::Reserved(v)
            | AvpPayloadType::Unassigned(v)
            | AvpPayloadType::Dynamic(v) => v,
        }
    }
}

impl From<u8> for AvpPayloadType {
    fn from(value: u8) -> Self {
        if (35..=71).contains(&value) || (77..=95).contains(&value) {
            return Self::Unassigned(value);
        }

        if (72..=76).contains(&value) {
            return Self::Reserved(value);
        }

        if (96..=127).contains(&value) {
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

        Self::Dynamic(value)
    }
}
