//! @see Table 0 - streamType Values of ISO/IEC 14496-1

use crate::codec::mpeg4_generic::errors::RtpMpeg4Error;

#[repr(u8)]
pub enum StreamType {
    Forbidden = 0x00,
    ObjectDescriptorStream = 0x01,
    ClockReferenceStream = 0x02,
    SceneDescriptionStream = 0x03,
    VisualStream = 0x04,
    AudioStream = 0x05,
    MPEG7Stream = 0x06,
    IPMPStream = 0x07,
    ObjectContentInfoStream = 0x08,
    MPEGJStream = 0x09,
    ISOReserved(u8),
    Private(u8),
}

impl TryFrom<u8> for StreamType {
    type Error = RtpMpeg4Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(StreamType::Forbidden),
            0x01 => Ok(StreamType::ObjectDescriptorStream),
            0x02 => Ok(StreamType::ClockReferenceStream),
            0x03 => Ok(StreamType::SceneDescriptionStream),
            0x04 => Ok(StreamType::VisualStream),
            0x05 => Ok(StreamType::AudioStream),
            0x06 => Ok(StreamType::MPEG7Stream),
            0x07 => Ok(StreamType::IPMPStream),
            0x08 => Ok(StreamType::ObjectContentInfoStream),
            0x09 => Ok(StreamType::MPEGJStream),
            0x0A..=0x1F => Ok(StreamType::ISOReserved(value)),
            0x20..=0x3F => Ok(StreamType::Private(value)),
            _ => Err(RtpMpeg4Error::InvalidStreamType(value)),
        }
    }
}

impl From<StreamType> for u8 {
    fn from(stream_type: StreamType) -> Self {
        match stream_type {
            StreamType::Forbidden => 0x00,
            StreamType::ObjectDescriptorStream => 0x01,
            StreamType::ClockReferenceStream => 0x02,
            StreamType::SceneDescriptionStream => 0x03,
            StreamType::VisualStream => 0x04,
            StreamType::AudioStream => 0x05,
            StreamType::MPEG7Stream => 0x06,
            StreamType::IPMPStream => 0x07,
            StreamType::ObjectContentInfoStream => 0x08,
            StreamType::MPEGJStream => 0x09,
            StreamType::ISOReserved(value) => value,
            StreamType::Private(value) => value,
        }
    }
}
