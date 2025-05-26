pub mod reader;
pub mod writer;

use codec_h264::avc_decoder_configuration_record::AvcDecoderConfigurationRecord;
use utils::traits::dynamic_sized_packet::DynamicSizedPacket;

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

#[derive(Debug, Clone)]
pub enum VideoConfig {
    H264 {
        sps: Option<codec_h264::sps::Sps>,
        pps: Option<codec_h264::pps::Pps>,
        sps_ext: Option<codec_h264::sps_ext::SpsExt>,
        avc_decoder_configuration_record:
            Option<codec_h264::avc_decoder_configuration_record::AvcDecoderConfigurationRecord>,
    },
    // TODO
}

impl From<AvcDecoderConfigurationRecord> for VideoConfig {
    fn from(value: AvcDecoderConfigurationRecord) -> Self {
        let sps = value
            .sequence_parameter_sets
            .first()
            .map(|v| v.parameter_set.clone());
        let pps = value
            .picture_parameter_sets
            .first()
            .map(|v| v.parameter_set.clone());

        VideoConfig::H264 {
            sps,
            pps,
            sps_ext: value
                .sps_ext_related
                .as_ref()
                .map(|v| v.sequence_parameter_set_ext.first())
                .unwrap_or_default()
                .map(|v| v.parameter_set.clone()),
            avc_decoder_configuration_record: Some(value),
        }
    }
}

impl From<&VideoConfig> for VideoCodecCommon {
    fn from(value: &VideoConfig) -> Self {
        match value {
            VideoConfig::H264 { .. } => VideoCodecCommon::AVC,
            // TODO
        }
    }
}

#[derive(Debug, Clone)]
pub enum VideoFrameUnit {
    H264 {
        nal_units: Vec<codec_h264::nalu::NalUnit>,
    },
    // TODO
}

impl VideoFrameUnit {
    pub fn units_cnt(&self) -> usize {
        match self {
            Self::H264 { nal_units } => nal_units.len(),
        }
    }

    pub fn bytes_cnt(&self, delimiter_size: usize) -> usize {
        match self {
            Self::H264 { nal_units } => nal_units
                .iter()
                .fold(0, |prev, item| prev + item.get_packet_bytes_count()),
        }
        .checked_add(self.units_cnt().checked_mul(delimiter_size).unwrap())
        .unwrap()
    }
}
