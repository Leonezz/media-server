use num::ToPrimitive;
use utils::traits::dynamic_sized_packet::DynamicSizedBitsPacket;

use crate::{
    exp_golomb::{find_se_bits_count, find_ue_bits_cound},
    scaling_list::SeqScalingMatrix,
};

pub mod reader;
pub mod writer;

#[derive(Debug, Clone)]
pub struct SliceGroupMapType0 {
    pub run_length_minus1: Vec<u64>, // ue(v), in [0, PicSizeInMapUnits - 1]
}

impl DynamicSizedBitsPacket for SliceGroupMapType0 {
    fn get_packet_bits_count(&self) -> usize {
        self.run_length_minus1
            .iter()
            .fold(0, |prev, item| prev + find_ue_bits_cound(*item).unwrap())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SliceGroupMaptype2Item {
    pub top_left: u64,     // ue(v)
    pub bottom_right: u64, // ue(v)
}

impl DynamicSizedBitsPacket for SliceGroupMaptype2Item {
    fn get_packet_bits_count(&self) -> usize {
        find_ue_bits_cound(self.top_left).unwrap() + find_ue_bits_cound(self.bottom_right).unwrap()
    }
}

#[derive(Debug, Clone)]
pub struct SliceGroupMapType2 {
    /// top_left[i] shall be less than or equal to bottom_right[i] and bottom_right[i] shall be less than PicSizeInMapUnits.
    /// (top_left[i] % PicWidthInMbs) shall be less than or equal to the value of (bottom_right[i] % PicWidthInMbs)
    pub items: Vec<SliceGroupMaptype2Item>,
}

impl DynamicSizedBitsPacket for SliceGroupMapType2 {
    fn get_packet_bits_count(&self) -> usize {
        self.items
            .iter()
            .fold(0, |prev, item| prev + item.get_packet_bits_count())
    }
}

#[derive(Debug, Clone)]
pub struct SliceGroupMapType345 {
    pub slice_group_change_direction_flag: bool, // u(1)
    pub slice_group_change_rate_minus1: u64,     // ue(v), in [0, PicSizeInMapUnits - 1]
}

impl DynamicSizedBitsPacket for SliceGroupMapType345 {
    fn get_packet_bits_count(&self) -> usize {
        1 + // slice_group_change_direction_flag
        find_ue_bits_cound(self.slice_group_change_rate_minus1).unwrap()
    }
}

#[derive(Debug, Clone)]
pub struct SliceGroupMapType6 {
    #[allow(unused)]
    pic_size_in_map_units_minus1: u64, // ue(v), equal to PicSizeInMapUnits − 1
    bits_cnt: u32,                // for slice_group_id
    pub slice_group_id: Vec<u64>, // u(v), v = Ceil(Log2(num_slice_groups_minus1 + 1)) bits., in [0, num_slice_groups_minus1]
}

impl DynamicSizedBitsPacket for SliceGroupMapType6 {
    fn get_packet_bits_count(&self) -> usize {
        find_ue_bits_cound(self.pic_size_in_map_units_minus1).unwrap()
            + self
                .slice_group_id
                .len()
                .checked_mul(self.bits_cnt.to_usize().unwrap())
                .unwrap()
    }
}

#[derive(Debug, Clone)]
pub enum SliceGroupMapTypeRelated {
    Type0(SliceGroupMapType0),
    Type2(SliceGroupMapType2),
    Type345(SliceGroupMapType345),
    Type6(SliceGroupMapType6),
}

impl DynamicSizedBitsPacket for SliceGroupMapTypeRelated {
    fn get_packet_bits_count(&self) -> usize {
        match self {
            Self::Type0(item) => item.get_packet_bits_count(),
            Self::Type2(item) => item.get_packet_bits_count(),
            Self::Type345(item) => item.get_packet_bits_count(),
            Self::Type6(item) => item.get_packet_bits_count(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NumSliceGroupsMinus1Positive {
    pub slice_group_map_type: u8, // ue(v), in [0, 6]
    pub slice_group_map_type_related: SliceGroupMapTypeRelated,
}

impl DynamicSizedBitsPacket for NumSliceGroupsMinus1Positive {
    fn get_packet_bits_count(&self) -> usize {
        find_ue_bits_cound(self.slice_group_map_type).unwrap()
            + self.slice_group_map_type_related.get_packet_bits_count()
    }
}

#[derive(Debug, Clone)]
pub struct MoreData {
    pub transform_8x8_flag: bool, // u(1)
    #[allow(unused)]
    pic_scaling_matrix_present_flag: bool, // u(1)
    pub pic_scaling_matrix: Option<SeqScalingMatrix>,
    pub second_chroma_qp_index_offset: i64, // se(v)
}

impl DynamicSizedBitsPacket for MoreData {
    fn get_packet_bits_count(&self) -> usize {
        1 + // transform_8x8_flag
        1 + // pic_scaling_matrix_present_flag
        self.pic_scaling_matrix.as_ref().map_or(0, |matrix| {
            let mut cnt = 0;
            for (i, present) in matrix.seq_scaling_list_present_flag.iter().enumerate() {
                if !*present {
                  continue;
                }
                if i < 6 {
                    cnt += matrix.scaling_list_4x4[i].get_packet_bits_count();
                } else {
                    cnt += matrix.scaling_list_8x8[i - 6].get_packet_bits_count();
                }
            }
            cnt
        }) +
        find_se_bits_count(self.second_chroma_qp_index_offset).unwrap()
    }
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

impl DynamicSizedBitsPacket for Pps {
    fn get_packet_bits_count(&self) -> usize {
        find_ue_bits_cound(self.pic_parameter_set_id).unwrap() +
        find_ue_bits_cound(self.seq_parameter_set_id).unwrap() +
        1 + // entropy_coding_mode_flag
        1 + // bottom_field_pic_order_in_frame_present_flag
        find_ue_bits_cound(self.num_slice_groups_minus1).unwrap() +
        self.num_slice_groups_minus1_positive.as_ref().map_or(0, |v| v.get_packet_bits_count()) +
        find_ue_bits_cound(self.num_ref_idx_10_default_active_minus1).unwrap() +
        find_ue_bits_cound(self.num_ref_idx_11_default_active_minus1).unwrap() +
        1 + // weighted_pred_flag
        2 + // weighted_bipred_idc
        find_se_bits_count(self.pic_init_qp_minus26).unwrap() +
        find_se_bits_count(self.pic_init_qs_minus26).unwrap() +
        find_se_bits_count(self.chroma_qp_index_offset).unwrap() +
        1 + // deblocking_filter_control_present_flag
        1 + // constrained_intra_pred_flag
        1 + // redudant_pic_cnt_present_flag
        self.more_data.as_ref().map_or(0, |v| v.get_packet_bits_count())
    }
}
