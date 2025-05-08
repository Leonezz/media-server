use bitstream_io::BitRead;
use num::ToPrimitive;
use utils::traits::reader::BitwiseReadFrom;

use crate::{
    errors::H264CodecError,
    exp_golomb::{read_se, read_ue},
    vui::VuiParameters,
};

use super::{
    FrameCropping, PicOrderCntType1, ProfileIdcRelated, SeqScalingMatrix, Sps,
    scaling_list::ScalingListRaw,
};

impl<R: BitRead> BitwiseReadFrom<R> for ProfileIdcRelated {
    type Error = H264CodecError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let chroma_format_idc = read_ue(&mut reader)?.to_u8().unwrap();
        let separate_colour_plane_flag = if chroma_format_idc == 3 {
            Some(reader.read_bit()?)
        } else {
            None
        };
        let bit_depth_luma_minus8 = read_ue(&mut reader)?;
        let bit_depth_chroma_minus8 = read_ue(&mut reader)?;
        let qpprime_y_zero_transform_bypass_flag = reader.read_bit()?;
        let seq_scaling_matrix_present_flag = reader.read_bit()?;
        let seq_scaling_matrix = if seq_scaling_matrix_present_flag {
            let cnt = if chroma_format_idc != 3 { 8 } else { 12 };
            let mut seq_scaling_list_present_flag = [false; 12];
            let mut use_default_scaling_matrix_4x4_flag = [false; 6]; // TODO: is this not needed?
            let mut scaling_list_4x4 = [ScalingListRaw::<16>::default(); 6];
            let mut use_default_scaling_matrix_8x8_flag = [false; 6]; // TODO: is this not needed?
            let mut scaling_list_8x8 = [ScalingListRaw::<64>::default(); 6];
            for i in 0..cnt {
                seq_scaling_list_present_flag[i] = reader.read_bit()?;
                if seq_scaling_list_present_flag[i] {
                    if i < 6 {
                        scaling_list_4x4[i] = ScalingListRaw::<16>::new(
                            &mut reader,
                            &mut use_default_scaling_matrix_4x4_flag[i],
                        )?;
                    } else {
                        scaling_list_8x8[i - 6] = ScalingListRaw::<64>::new(
                            &mut reader,
                            &mut use_default_scaling_matrix_8x8_flag[i - 6],
                        )?;
                    }
                }
            }
            Some(SeqScalingMatrix {
                scaling_list_4x4,
                scaling_list_8x8,
                seq_scaling_list_present_flag,
            })
        } else {
            None
        };
        Ok(Self {
            chroma_format_idc,
            separate_colour_plane_flag,
            bit_depth_luma_minus8,
            bit_depth_chroma_minus8,
            qpprime_y_zero_transform_bypass_flag,
            seq_scaling_matrix_present_flag,
            seq_scaling_matrix,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for PicOrderCntType1 {
    type Error = H264CodecError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let delta_pic_order_always_zero_flag = reader.read_bit()?;
        let offset_for_non_ref_pic = read_se(&mut reader)?;
        let offset_for_top_to_bottom_field = read_se(&mut reader)?;
        let num_ref_frames_in_pic_order_cnt_cycle = read_ue(&mut reader)?;
        let mut offset_for_ref_frame =
            vec![0; num_ref_frames_in_pic_order_cnt_cycle.to_usize().unwrap()];
        offset_for_ref_frame.iter_mut().try_for_each(|item| {
            *item = read_se(&mut reader)?;
            Ok::<(), Self::Error>(())
        })?;
        Ok(Self {
            delta_pic_order_always_zero_flag,
            offset_for_non_ref_pic,
            offset_for_top_to_bottom_field,
            num_ref_frames_in_pic_order_cnt_cycle,
            offset_for_ref_frame,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for FrameCropping {
    type Error = H264CodecError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let frame_crop_left_offset = read_ue(&mut reader)?;
        let frame_crop_right_offset = read_ue(&mut reader)?;
        let frame_crop_top_offset = read_ue(&mut reader)?;
        let frame_crop_bottom_offset = read_ue(&mut reader)?;
        Ok(Self {
            frame_crop_left_offset,
            frame_crop_right_offset,
            frame_crop_top_offset,
            frame_crop_bottom_offset,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for Sps {
    type Error = H264CodecError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let profile_idc = reader.read::<8, u8>()?;
        let constraint_set0_flag = reader.read_bit()?;
        let constraint_set1_flag = reader.read_bit()?;
        let constraint_set2_flag = reader.read_bit()?;
        let constraint_set3_flag = reader.read_bit()?;
        let constraint_set4_flag = reader.read_bit()?;
        let constraint_set5_flag = reader.read_bit()?;
        let reserved_zero_2bits = reader.read::<2, u8>()?;
        if reserved_zero_2bits != 0 {
            return Err(H264CodecError::SyntaxError(format!(
                "reserved_zero_2bits in sps should be 0: {}",
                reserved_zero_2bits
            )));
        }
        let level_idc = reader.read::<8, u8>()?;
        let seq_parameter_set_id = read_ue(&mut reader)?.to_u8().unwrap();
        let profile_idc_related = if [100, 110, 122, 244, 44, 83, 86, 118, 128, 138, 139, 134, 135]
            .contains(&profile_idc)
        {
            Some(ProfileIdcRelated::read_from(&mut reader)?)
        } else {
            None
        };
        let log2_max_frame_num_minus4 = read_ue(&mut reader)?;
        let pic_order_cnt_type = read_ue(&mut reader)?;
        let log2_max_pic_order_cnt_lsb_minus4 = if pic_order_cnt_type == 0 {
            Some(read_ue(&mut reader)?)
        } else {
            None
        };
        let pic_order_cnt_type_1 = if pic_order_cnt_type == 1 {
            Some(PicOrderCntType1::read_from(&mut reader)?)
        } else {
            None
        };
        let max_num_ref_frames = read_ue(&mut reader)?;
        let gaps_in_frame_num_value_allowed_flag = reader.read_bit()?;
        let pic_width_in_mbs_minus1 = read_ue(&mut reader)?;
        let pic_height_in_map_units_minus1 = read_ue(&mut reader)?;
        let frame_mbs_only_flag = reader.read_bit()?;
        let mb_adaptive_frame_field_flag = if frame_mbs_only_flag {
            Some(reader.read_bit()?)
        } else {
            None
        };
        let direct_8x8_inference_flag = reader.read_bit()?;
        let frame_cropping_flag = reader.read_bit()?;
        let frame_cropping = if frame_cropping_flag {
            Some(FrameCropping::read_from(&mut reader)?)
        } else {
            None
        };
        let vui_parameters_present_flag = reader.read_bit()?;
        let vui_parameters = if vui_parameters_present_flag {
            Some(VuiParameters::read_from(&mut reader)?)
        } else {
            None
        };
        Ok(Self {
            profile_idc,
            constraint_set0_flag,
            constraint_set1_flag,
            constraint_set2_flag,
            constraint_set3_flag,
            constraint_set4_flag,
            constraint_set5_flag,
            reserved_zero_2bits,
            level_idc,
            seq_parameter_set_id,
            profile_idc_related,
            log2_max_frame_num_minus4,
            pic_order_cnt_type,
            log2_max_pic_order_cnt_lsb_minus4,
            pic_order_cnt_type_1,
            max_num_ref_frames,
            gaps_in_frame_num_value_allowed_flag,
            pic_width_in_mbs_minus1,
            pic_height_in_map_units_minus1,
            frame_mbs_only_flag,
            mb_adaptive_frame_field_flag,
            direct_8x8_inference_flag,
            frame_cropping_flag,
            frame_cropping,
            vui_parameters_present_flag,
            vui_parameters,
        })
    }
}
