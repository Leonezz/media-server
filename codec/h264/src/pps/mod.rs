use crate::scaling_list::SeqScalingMatrix;

pub mod reader;
pub mod writer;

#[derive(Debug, Clone)]
pub struct SliceGroupMapType0 {
    pub run_length_minus1: Vec<u64>, // ue(v), in [0, PicSizeInMapUnits - 1]
}

#[derive(Debug, Clone, Copy)]
pub struct SliceGroupMaptype2Item {
    pub top_left: u64,     // ue(v)
    pub bottom_right: u64, // ue(v)
}

#[derive(Debug, Clone)]
pub struct SliceGroupMapType2 {
    /// top_left[i] shall be less than or equal to bottom_right[i] and bottom_right[i] shall be less than PicSizeInMapUnits.
    /// (top_left[i] % PicWidthInMbs) shall be less than or equal to the value of (bottom_right[i] % PicWidthInMbs)
    pub items: Vec<SliceGroupMaptype2Item>,
}

#[derive(Debug, Clone)]
pub struct SliceGroupMapType345 {
    pub slice_group_change_direction_flag: bool, // u(1)
    pub slice_group_change_rate_minus1: u64,     // ue(v), in [0, PicSizeInMapUnits - 1]
}

#[derive(Debug, Clone)]
pub struct SliceGroupMapType6 {
    #[allow(unused)]
    pic_size_in_map_units_minus1: u64, // ue(v), equal to PicSizeInMapUnits − 1
    bits_cnt: u32,                // for slice_group_id
    pub slice_group_id: Vec<u64>, // u(v), v = Ceil(Log2(num_slice_groups_minus1 + 1)) bits., in [0, num_slice_groups_minus1]
}

#[derive(Debug, Clone)]
pub enum SliceGroupMapTypeRelated {
    Type0(SliceGroupMapType0),
    Type2(SliceGroupMapType2),
    Type345(SliceGroupMapType345),
    Type6(SliceGroupMapType6),
}

#[derive(Debug, Clone)]
pub struct NumSliceGroupsMinus1Positive {
    pub slice_group_map_type: u8, // ue(v), in [0, 6]
    pub slice_group_map_type_related: SliceGroupMapTypeRelated,
}

#[derive(Debug, Clone)]
pub struct MoreData {
    pub transform_8x8_flag: bool, // u(1)
    #[allow(unused)]
    pic_scaling_matrix_present_flag: bool, // u(1)
    pub pic_scaling_matrix: Option<SeqScalingMatrix>,
    pub second_chroma_qp_index_offset: i64, // se(v)
}

/// @see: Recommendation  ITU-T H.264 (V15) (08/2024)   – Coding of moving video
/// Section 7.3.2.2 Picture parameter set RBSP syntax
#[derive(Debug, Clone)]
pub struct Pps {
    pub pic_parameter_set_id: u8,       // ue(v), in [0, 255]
    pub seq_parameter_set_id: u8,       // ue(v), in [0, 31]
    pub entropy_coding_mode_flag: bool, // u(1)
    pub bottom_field_pic_order_in_frame_present_flag: bool, // u(1)
    pub num_slice_groups_minus1: u64,   // ue(v)
    /// if num_slice_groups_minus1 > 0 {
    pub num_slice_groups_minus1_positive: Option<NumSliceGroupsMinus1Positive>,
    /// }
    pub num_ref_idx_10_default_active_minus1: u8, // ue(v), in [0, 31]
    pub num_ref_idx_11_default_active_minus1: u8, // ue(v), in [0, 31]
    pub weighted_pred_flag: bool,                 // u(1)
    pub weighted_bipred_idc: u8,                  // u(2), in [0, 2]
    pub pic_init_qp_minus26: i8,                  // se(v), in [−(26 + QpBdOffsetY), 25]
    pub pic_init_qs_minus26: i8,                  // se(v), in [-26, 25]
    pub chroma_qp_index_offset: i8,               // se(v), in [-12, 12]
    pub deblocking_filter_control_present_flag: bool, // u(1)
    pub constrained_intra_pred_flag: bool,        // u(1)
    pub redundant_pic_cnt_present_flag: bool,     // u(1)
    pub more_data: Option<MoreData>,
}
