use scaling_list::ScalingListRaw;

use crate::vui::VuiParameters;

pub mod reader;
pub mod scaling_list;
pub mod writer;

/// @see: Recommendation  ITU-T H.264 (V15) (08/2024)   – Coding of moving video
/// Section 7.3.2.1.1 Sequence parameter set data syntax

#[derive(Debug)]
pub struct SeqScalingMatrix {
    seq_scaling_list_present_flag: [bool; 12], // u(1)
    /// if seq_scaling_list_present_flag[i]
    pub scaling_list_4x4: [ScalingListRaw<16>; 6], // TODO-
    pub scaling_list_8x8: [ScalingListRaw<64>; 6],
}

#[derive(Debug)]
pub struct ProfileIdcRelated {
    pub chroma_format_idc: u8, // ue(v), should be in [0, 3]
    /// if( chroma_format_idc == 3 ) {
    pub separate_colour_plane_flag: Option<bool>, // u(1)
    pub bit_depth_luma_minus8: u64, // ue(v)
    pub bit_depth_chroma_minus8: u64, // ue(v)
    pub qpprime_y_zero_transform_bypass_flag: bool, // u(1)
    #[allow(unused)]
    seq_scaling_matrix_present_flag: bool, // u(1),
    /// if seq_scaling_matrix_present_flag {
    pub seq_scaling_matrix: Option<SeqScalingMatrix>,
    // }
}

#[derive(Debug)]
pub struct PicOrderCntType1 {
    pub delta_pic_order_always_zero_flag: bool, // u(1)
    pub offset_for_non_ref_pic: i64,            // se(v)
    pub offset_for_top_to_bottom_field: i64,    // se(v)
    #[allow(unused)]
    num_ref_frames_in_pic_order_cnt_cycle: u64, // ue(v)
    pub offset_for_ref_frame: Vec<i64>,         // se(v)
}

#[derive(Debug)]
pub struct FrameCropping {
    pub frame_crop_left_offset: u64,   // ue(v)
    pub frame_crop_right_offset: u64,  // ue(v)
    pub frame_crop_top_offset: u64,    // ue(v)
    pub frame_crop_bottom_offset: u64, // ue(v)
}

#[derive(Debug)]
pub struct Sps {
    pub profile_idc: u8,            // u(8)
    pub constraint_set0_flag: bool, // u(1)
    pub constraint_set1_flag: bool, // u(1)
    pub constraint_set2_flag: bool, // u(1)
    pub constraint_set3_flag: bool, // u(1)
    pub constraint_set4_flag: bool, // u(1)
    pub constraint_set5_flag: bool, // u(1)
    #[allow(unused)]
    reserved_zero_2bits: u8, // u(2), equal to 0
    pub level_idc: u8,              // u(8)
    pub seq_parameter_set_id: u8,   // ue(v), value shoud be in [0, 31]
    /// if( profile_idc == 100 || profile_idc == 110 ||
    ///     profile_idc == 122 || profile_idc == 244 || profile_idc == 44 ||
    ///     profile_idc == 83  || profile_idc == 86  || profile_idc == 118 ||
    ///     profile_idc == 128 || profile_idc == 138 || profile_idc == 139 ||
    ///     profile_idc == 134 || profile_idc == 135 ) {
    ///
    pub profile_idc_related: Option<ProfileIdcRelated>,
    /// }
    pub log2_max_frame_num_minus4: u64, // ue(v)
    pub pic_order_cnt_type: u64, // ue(v)
    /// if pic_order_cnt_type == 0 {
    pub log2_max_pic_order_cnt_lsb_minus4: Option<u64>, // ue(v)
    /// } else if pic_order_cnt_type == 1 {
    pub pic_order_cnt_type_1: Option<PicOrderCntType1>,
    /// }
    pub max_num_ref_frames: u64, // ue(v),
    pub gaps_in_frame_num_value_allowed_flag: bool, // u(1)
    pub pic_width_in_mbs_minus1: u64,               // ue(v)
    pub pic_height_in_map_units_minus1: u64,        // ue(v)
    #[allow(unused)]
    frame_mbs_only_flag: bool,  // u(1),
    /// if frame_mbs_only_flag {
    pub mb_adaptive_frame_field_flag: Option<bool>, // u(1)
    /// }
    pub direct_8x8_inference_flag: bool,   // u(1)
    #[allow(unused)]
    frame_cropping_flag: bool,  // u(1)
    /// if frame_cropping_flag {
    pub frame_cropping: Option<FrameCropping>,
    /// }
    #[allow(unused)]
    vui_parameters_present_flag: bool, // u(1)
    /// if vui_parameters_present_flag {
    pub vui_parameters: Option<VuiParameters>,
    // }
}
