use hrd_parameters::HrdParameters;

use crate::errors::H264CodecError;

pub mod hrd_parameters;
pub mod reader;
pub mod writer;

#[derive(Debug, Clone, Copy)]
pub struct AspectRatioInfoExtendedSAR {
    pub sar_width: u16,  // u(16)
    pub sar_height: u16, // u(16)
}

/// @see: Table E-1 – Meaning of sample aspect ratio indicator
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AspectRatioIdc {
    Unspecified = 0,
    Square = 1,
    Ratio12_11 = 2,
    Ratio10_11 = 3,
    Ratio16_11 = 4,
    Ratio40_30 = 5,
    Ratio24_11 = 6,
    Ratio20_11 = 7,
    Ratio32_11 = 8,
    Ratio80_33 = 9,
    Ratio18_11 = 10,
    Ratio15_11 = 11,
    Ratio64_33 = 12,
    Ratio160_99 = 13,
    Ratio4_3 = 14,
    Ratio3_2 = 15,
    Ratio2_1 = 16,
    Reserved(u8),
    ExtendedSAR = 255,
}

impl From<u8> for AspectRatioIdc {
    fn from(value: u8) -> Self {
        match value {
            0 => AspectRatioIdc::Unspecified,
            1 => AspectRatioIdc::Square,
            2 => AspectRatioIdc::Ratio12_11,
            3 => AspectRatioIdc::Ratio10_11,
            4 => AspectRatioIdc::Ratio16_11,
            5 => AspectRatioIdc::Ratio40_30,
            6 => AspectRatioIdc::Ratio24_11,
            7 => AspectRatioIdc::Ratio20_11,
            8 => AspectRatioIdc::Ratio32_11,
            9 => AspectRatioIdc::Ratio80_33,
            10 => AspectRatioIdc::Ratio18_11,
            11 => AspectRatioIdc::Ratio15_11,
            12 => AspectRatioIdc::Ratio64_33,
            13 => AspectRatioIdc::Ratio160_99,
            14 => AspectRatioIdc::Ratio4_3,
            15 => AspectRatioIdc::Ratio3_2,
            16 => AspectRatioIdc::Ratio2_1,
            255 => AspectRatioIdc::ExtendedSAR,
            reserved => AspectRatioIdc::Reserved(reserved),
        }
    }
}

impl From<AspectRatioIdc> for u8 {
    fn from(value: AspectRatioIdc) -> Self {
        match value {
            AspectRatioIdc::Unspecified => 0,
            AspectRatioIdc::Square => 1,
            AspectRatioIdc::Ratio12_11 => 2,
            AspectRatioIdc::Ratio10_11 => 3,
            AspectRatioIdc::Ratio16_11 => 4,
            AspectRatioIdc::Ratio40_30 => 5,
            AspectRatioIdc::Ratio24_11 => 6,
            AspectRatioIdc::Ratio20_11 => 7,
            AspectRatioIdc::Ratio32_11 => 8,
            AspectRatioIdc::Ratio80_33 => 9,
            AspectRatioIdc::Ratio18_11 => 10,
            AspectRatioIdc::Ratio15_11 => 11,
            AspectRatioIdc::Ratio64_33 => 12,
            AspectRatioIdc::Ratio160_99 => 13,
            AspectRatioIdc::Ratio4_3 => 14,
            AspectRatioIdc::Ratio3_2 => 15,
            AspectRatioIdc::Ratio2_1 => 16,
            AspectRatioIdc::ExtendedSAR => 255,
            AspectRatioIdc::Reserved(value) => value,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AspectRatioInfo {
    pub aspect_ratio_idc: AspectRatioIdc, // u(8), see: Table E-1 – Meaning of sample aspect ratio indicator
    /// if aspect_ratio_idc == Extended_SAR {
    pub aspect_ratio_info_extended_sar: Option<AspectRatioInfoExtendedSAR>, // }
}

#[derive(Debug, Clone, Copy)]
pub struct ColourDescription {
    pub colour_primaries: u8, // u(8), see: Table E-3 – Colour primaries interpretation using colour_primaries syntax element
    pub transfer_characteristics: u8, // u(8), see: Table E-4 – Transfer characteristics interpretation using transfer_characteristics syntax element
    pub matrix_coefficients: u8, // u(8), see: Table E-5 – Matrix coefficients interpretation using the matrix_coefficients syntax element
}

/// @see: Table E-2 – Meaning of video_format
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoFormat {
    Component = 0,
    PAL = 1,
    NTSC = 2,
    SECAM = 3,
    MAC = 4,
    Unspecified = 5,
    Reserved6 = 6,
    Reserved7 = 7,
}

impl TryFrom<u8> for VideoFormat {
    type Error = H264CodecError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(VideoFormat::Component),
            1 => Ok(VideoFormat::PAL),
            2 => Ok(VideoFormat::NTSC),
            3 => Ok(VideoFormat::SECAM),
            4 => Ok(VideoFormat::MAC),
            5 => Ok(VideoFormat::Unspecified),
            6 => Ok(VideoFormat::Reserved6),
            7 => Ok(VideoFormat::Reserved7),
            _ => Err(H264CodecError::UnknownVideoFormat(value)),
        }
    }
}

impl From<VideoFormat> for u8 {
    fn from(value: VideoFormat) -> Self {
        match value {
            VideoFormat::Component => 0,
            VideoFormat::PAL => 1,
            VideoFormat::NTSC => 2,
            VideoFormat::SECAM => 3,
            VideoFormat::MAC => 4,
            VideoFormat::Unspecified => 5,
            VideoFormat::Reserved6 => 6,
            VideoFormat::Reserved7 => 7,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VideoSignalType {
    pub video_format: VideoFormat, // u(3), see: Table E-2 – Meaning of video_format
    pub video_full_range_flag: bool, // u(1)
    #[allow(unused)]
    pub(crate) colour_description_present_flag: bool, // u(1)
    /// colour_description_present_flag {
    pub colour_description: Option<ColourDescription>,
    // }
}

#[derive(Debug, Clone, Copy)]
pub struct ChromaLocInfo {
    pub chroma_sample_loc_type_top_field: u8, // ue(v), in [0, 5]
    pub chroma_sample_loc_type_bottom_field: u8, // ue(v), in [0, 5]
}

#[derive(Debug, Clone)]
pub struct TimingInfo {
    pub num_units_in_tick: u32,      // u(32), in (0, )
    pub time_scale: u32,             // u(32), in (0, )
    pub fixed_frame_rate_flag: bool, // u(1)
}

#[derive(Debug, Clone)]
pub struct BitstreamRestriction {
    pub motion_vectors_over_pic_boundaries_flag: bool, // u(1),
    pub max_bytes_per_pic_denom: u8,                   // ue(v), in [0, 16]
    pub max_bits_per_mb_denom: u8,                     // ue(v), in [0, 16]
    pub log2_max_mv_length_horizontal: u8,             // ue(v), in [0, 15]
    pub log2_max_mv_length_vertical: u8,               // ue(v), in [0, 15]
    pub max_num_reorder_frames: u64,                   // ue(v), in [0, max_dec_frame_buffering]
    pub max_dec_frame_buffering: u64,                  // ue(v)
}

/// @see: Recommendation  ITU-T H.264 (V15) (08/2024)   – Coding of moving video
/// Section E.1.1 VUI parameters syntax
#[derive(Debug, Clone)]
pub struct VuiParameters {
    #[allow(unused)]
    aspect_ratio_info_present_flag: bool, // u(1)
    /// if aspect_ratio_info_present_flag {
    pub aspect_ratio_info: Option<AspectRatioInfo>,
    /// }
    #[allow(unused)]
    overscan_info_present_flag: bool, // u(1)
    /// if overscan_info_present_flag {
    pub overscan_appropriate_flag: Option<bool>, // u(1)
    /// }
    #[allow(unused)]
    video_signal_type_present_flag: bool, // u(1)
    pub video_signal_type: Option<VideoSignalType>,
    #[allow(unused)]
    chroma_loc_info_present_flag: bool, // u(1)
    /// if chroma_loc_info_present_flag {
    pub chroma_loc_info: Option<ChromaLocInfo>,
    /// }
    #[allow(unused)]
    timing_info_present_flag: bool, // u(1)
    /// if timing_info_present_flag {
    pub timing_info: Option<TimingInfo>,
    /// }
    #[allow(unused)]
    nal_hrd_parameters_present_flag: bool, // u(1)
    /// if nal_hrd_parameters_present_flag {
    pub nal_hrd_parameters: Option<HrdParameters>,
    /// }
    #[allow(unused)]
    vcl_hrd_parameters_present_flag: bool, // u(1)
    /// if vcl_hrd_parameters_present_flag {
    pub vcl_hrd_parameters: Option<HrdParameters>,
    /// }
    /// if nal_hrd_parameters_present_flag || vcl_hrd_parameters_present_flag {
    pub low_delay_hrd_flag: Option<bool>, // u(1)
    /// }
    pub pic_struct_present_flag: bool, // u(1)
    #[allow(unused)]
    bitstream_restriction_flag: bool, // u(1)
    /// if bitstream_restriction_flag {
    pub bitstream_restriction: Option<BitstreamRestriction>,
    // }
}
