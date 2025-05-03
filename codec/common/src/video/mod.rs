use crate::FrameType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoCodecCommon {
    SorensonH263,
    ScreenVideo,
    On2VP6,
    On2VP6WithAlpha,
    ScreenVideoV2,
    AVC,
    HEVC,
    VP8,
    VP9,
    AV1,
}

#[derive(Debug, Clone)]
pub struct VideoFrameInfo {
    pub codec_id: VideoCodecCommon,
    pub frame_type: FrameType,
    pub timestamp_nano: u64,
}

impl VideoFrameInfo {
    pub fn new(codec_id: VideoCodecCommon, frame_type: FrameType, timestamp_nano: u64) -> Self {
        Self {
            codec_id,
            frame_type,
            timestamp_nano,
        }
    }
}
