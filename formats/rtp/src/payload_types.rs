use crate::rtcp::payload_types::RtcpPayloadType;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadType {
    Rtcp(RtcpPayloadType),
    Unspecified(u8),
}

pub mod rtp_payload_type {
    use codec_common::{audio::AudioCodecCommon, video::VideoCodecCommon};

    pub const MGEP4_AUDIO: u8 = 97;
    pub const H264_VIDEO: u8 = 96;

    pub fn get_rtp_payload_type(encoding_name: &str) -> Option<u8> {
        match encoding_name.to_lowercase().as_str() {
            "mpeg4-generic" => Some(MGEP4_AUDIO),
            "aac" => Some(MGEP4_AUDIO),
            "h264" => Some(H264_VIDEO),
            _ => None,
        }
    }

    pub fn get_rtp_clockrate(encoding_name: &str) -> Option<u64> {
        match encoding_name.to_lowercase().as_str() {
            "mpeg4-generic" => Some(1000),
            "aac" => Some(1000),
            "h264" => Some(90000),
            _ => None,
        }
    }

    pub fn audio_get_rtp_encoding_name(codec: AudioCodecCommon) -> Option<&'static str> {
        match codec {
            AudioCodecCommon::AAC => Some("mpeg4-generic"),
            _ => None,
        }
    }

    pub fn get_audio_rtp_payload_type(codec: AudioCodecCommon) -> Option<u8> {
        audio_get_rtp_encoding_name(codec).and_then(get_rtp_payload_type)
    }

    pub fn audio_get_rtp_clockrate(codec: AudioCodecCommon) -> Option<u64> {
        audio_get_rtp_encoding_name(codec).and_then(get_rtp_clockrate)
    }

    pub fn video_get_rtp_encoding_name(codec: VideoCodecCommon) -> Option<&'static str> {
        match codec {
            VideoCodecCommon::AVC => Some("h264"),
            _ => None,
        }
    }

    pub fn get_video_rtp_payload_type(codec: VideoCodecCommon) -> Option<u8> {
        video_get_rtp_encoding_name(codec).and_then(get_rtp_payload_type)
    }

    pub fn video_get_rtp_clockrate(codec: VideoCodecCommon) -> Option<u64> {
        video_get_rtp_encoding_name(codec).and_then(get_rtp_clockrate)
    }
}
