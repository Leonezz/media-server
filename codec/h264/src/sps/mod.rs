use bitstream_io::BitWrite;
use chroma_format_idc::ChromaFormatIdc;
use tokio_util::bytes::{BufMut, Bytes, BytesMut};
use utils::traits::{dynamic_sized_packet::DynamicSizedBitsPacket, writer::BitwiseWriteTo};

use crate::{
    exp_golomb::{find_se_bits_count, find_ue_bits_count},
    nalu::NalUnit,
    nalu_header::NaluHeader,
    rbsp::raw_bytes_to_rbsp,
    scaling_list::SeqScalingMatrix,
    vui::VuiParameters,
};

pub mod chroma_format_idc;
pub mod reader;
#[cfg(test)]
mod sps_test;
pub mod writer;
#[derive(Debug, Clone)]
pub struct ProfileIdcRelated {
    pub chroma_format_idc: ChromaFormatIdc, // ue(v), should be in [0, 3]
    /// if( chroma_format_idc == 3 ) {
    pub separate_colour_plane_flag: Option<bool>, // u(1)
    pub bit_depth_luma_minus8: u64,         // ue(v)
    pub bit_depth_chroma_minus8: u64,       // ue(v)
    pub qpprime_y_zero_transform_bypass_flag: bool, // u(1)
    #[allow(unused)]
    seq_scaling_matrix_present_flag: bool, // u(1),
    /// if seq_scaling_matrix_present_flag {
    pub seq_scaling_matrix: Option<SeqScalingMatrix>,
    // }
}

impl DynamicSizedBitsPacket for ProfileIdcRelated {
    fn get_packet_bits_count(&self) -> usize {
        let mut result = find_ue_bits_count(Into::<u8>::into(self.chroma_format_idc)).unwrap();
        if self.separate_colour_plane_flag.is_some() {
            result += 1;
        }
        result += find_ue_bits_count(self.bit_depth_luma_minus8).unwrap();
        result += find_ue_bits_count(self.bit_depth_chroma_minus8).unwrap();
        result += 1; // qpprime_y_zero_transform_bypass_flag
        result += 1; // seq_scaling_matrix_present_flag
        if let Some(matrix) = &self.seq_scaling_matrix {
            let cnt = if !matches!(self.chroma_format_idc, ChromaFormatIdc::Chroma444) {
                8
            } else {
                12
            };
            for i in 0..cnt {
                if matrix.seq_scaling_list_present_flag[i] {
                    if i < 6 {
                        result += matrix.scaling_list_4x4[i].get_packet_bits_count();
                    } else {
                        result += matrix.scaling_list_8x8[i - 6].get_packet_bits_count();
                    }
                }
            }
        }
        result
    }
}

#[derive(Debug, Clone)]
pub struct PicOrderCntType1 {
    pub delta_pic_order_always_zero_flag: bool, // u(1)
    pub offset_for_non_ref_pic: i64,            // se(v)
    pub offset_for_top_to_bottom_field: i64,    // se(v)
    #[allow(unused)]
    num_ref_frames_in_pic_order_cnt_cycle: u64, // ue(v)
    pub offset_for_ref_frame: Vec<i64>,         // se(v)
}

impl DynamicSizedBitsPacket for PicOrderCntType1 {
    fn get_packet_bits_count(&self) -> usize {
        1 + // delta_pic_order_always_zero_flag
            find_se_bits_count(self.offset_for_non_ref_pic).unwrap() +
            find_se_bits_count(self.offset_for_top_to_bottom_field).unwrap() +
            find_ue_bits_count(self.num_ref_frames_in_pic_order_cnt_cycle).unwrap() +
            self.offset_for_ref_frame.iter().fold(0, |prev, item| prev + find_se_bits_count(*item).unwrap())
    }
}

#[derive(Debug, Clone)]
pub struct FrameCropping {
    pub frame_crop_left_offset: u64,   // ue(v)
    pub frame_crop_right_offset: u64,  // ue(v)
    pub frame_crop_top_offset: u64,    // ue(v)
    pub frame_crop_bottom_offset: u64, // ue(v)
}

impl DynamicSizedBitsPacket for FrameCropping {
    fn get_packet_bits_count(&self) -> usize {
        find_ue_bits_count(self.frame_crop_left_offset).unwrap()
            + find_ue_bits_count(self.frame_crop_right_offset).unwrap()
            + find_ue_bits_count(self.frame_crop_top_offset).unwrap()
            + find_ue_bits_count(self.frame_crop_bottom_offset).unwrap()
    }
}

/// @see: Recommendation  ITU-T H.264 (V15) (08/2024)   – Coding of moving video
/// Section 7.3.2.1.1 Sequence parameter set data syntax
#[derive(Debug, Clone)]
pub struct Sps {
    pub profile_idc: u8,            // u(8)
    pub constraint_set0_flag: bool, // u(1)
    pub constraint_set1_flag: bool, // u(1)
    pub constraint_set2_flag: bool, // u(1)
    pub constraint_set3_flag: bool, // u(1)
    pub constraint_set4_flag: bool, // u(1)
    pub constraint_set5_flag: bool, // u(1)
    pub reserved_zero_2bits: u8,    // u(2), equal to 0
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

impl Sps {
    pub fn get_video_height(&self) -> u64 {
        2_u64
            .checked_sub(self.frame_mbs_only_flag as u64)
            .and_then(|v| {
                v.checked_mul(self.pic_height_in_map_units_minus1.checked_add(1).unwrap())
            })
            .and_then(|v| v.checked_mul(16))
            .and_then(|v| {
                v.checked_sub(self.frame_cropping.as_ref().map_or(0, |v| {
                    v.frame_crop_top_offset
                        .checked_add(v.frame_crop_bottom_offset)
                        .and_then(|v| v.checked_mul(2))
                        .unwrap()
                }))
            })
            .unwrap()
    }

    pub fn get_video_width(&self) -> u64 {
        self.pic_width_in_mbs_minus1
            .checked_add(1)
            .and_then(|v| v.checked_mul(16))
            .and_then(|v| {
                v.checked_sub(self.frame_cropping.as_ref().map_or(0, |v| {
                    v.frame_crop_left_offset
                        .checked_add(v.frame_crop_right_offset)
                        .and_then(|v| v.checked_mul(2))
                        .unwrap()
                }))
            })
            .unwrap()
    }
}

impl DynamicSizedBitsPacket for Sps {
    fn get_packet_bits_count(&self) -> usize {
        8  + // profile_idc
        1 + // constraint_set0_flag
        1 + // constraint_set1_flag
        1 + // constraint_set2_flag
        1 + // constraint_set3_flag
        1 + // constraint_set4_flag
        1 + // constraint_set5_flag
        2 + // reserved_zero_2bits
        8 + // level_idc
        find_ue_bits_count(self.seq_parameter_set_id).unwrap() +
        self
            .profile_idc_related
            .as_ref()
            .map_or(0, |v| v.get_packet_bits_count()) +
        find_ue_bits_count(self.log2_max_frame_num_minus4).unwrap() +
        find_ue_bits_count(self.pic_order_cnt_type).unwrap() +
        self
            .log2_max_pic_order_cnt_lsb_minus4
            .map_or(0, |v| find_ue_bits_count(v).unwrap()) +
        self
            .pic_order_cnt_type_1
            .as_ref()
            .map_or(0, |v| v.get_packet_bits_count()) +
        find_ue_bits_count(self.max_num_ref_frames).unwrap() +
        1 + // gaps_in_frame_num_value_allowed_flag 
        find_ue_bits_count(self.pic_width_in_mbs_minus1).unwrap() +
        find_ue_bits_count(self.pic_height_in_map_units_minus1).unwrap() +
        1 + // frame_mbs_only_flag
        self.mb_adaptive_frame_field_flag.map_or(0, |_| 1) +
        1 + // direct_8x8_inference_flag
        1 + // frame_cropping_flag
        self
            .frame_cropping
            .as_ref()
            .map_or(0, |v| v.get_packet_bits_count()) +
        1 + // vui_parameters_present_flag
        self
            .vui_parameters
            .as_ref()
            .map_or(0, |v| v.get_packet_bits_count())
    }
}

impl From<&Sps> for NalUnit {
    fn from(value: &Sps) -> Self {
        let mut bytes: BytesMut = BytesMut::zeroed(
            value
                .get_packet_bits_count()
                .checked_add(4)
                .and_then(|v| v.checked_div(8))
                .unwrap(),
        );
        let mut writer =
            bitstream_io::BitWriter::endian(bytes.as_mut().writer(), bitstream_io::BigEndian);
        value.write_to(writer.by_ref()).unwrap();
        writer.byte_align().unwrap();
        let bytes = raw_bytes_to_rbsp(&bytes);
        Self {
            header: NaluHeader {
                forbidden_zero_bit: false,
                nal_ref_idc: 3,
                nal_unit_type: crate::nalu_type::NALUType::SPS,
            },
            body: Bytes::from_owner(bytes),
        }
    }
}
