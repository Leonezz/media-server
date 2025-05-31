use bitstream_io::BitWrite;
use num::ToPrimitive;
use utils::traits::writer::BitwiseWriteTo;

use crate::{
    errors::H264CodecError,
    exp_golomb::{write_se, write_ue},
};

use super::{
    FrameCropping, PicOrderCntType1, ProfileIdcRelated, Sps, chroma_format_idc::ChromaFormatIdc,
};

impl<W: BitWrite> BitwiseWriteTo<W> for ProfileIdcRelated {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        write_ue(writer, Into::<u8>::into(self.chroma_format_idc))?;
        if let Some(separate_colour_plane_flag) = self.separate_colour_plane_flag {
            writer.write_bit(separate_colour_plane_flag)?;
        }
        write_ue(writer, self.bit_depth_luma_minus8)?;
        write_ue(writer, self.bit_depth_chroma_minus8)?;
        writer.write_bit(self.qpprime_y_zero_transform_bypass_flag)?;
        if let Some(seq_scaling_matrix) = &self.seq_scaling_matrix {
            writer.write_bit(true)?; // seq_scaling_matrix_present_flag
            let cnt = if !matches!(self.chroma_format_idc, ChromaFormatIdc::Chroma444) {
                8
            } else {
                12
            };
            for i in 0..cnt {
                if seq_scaling_matrix.seq_scaling_list_present_flag[i] {
                    if i < 6 {
                        seq_scaling_matrix.scaling_list_4x4[i].write_to(writer)?;
                    } else {
                        seq_scaling_matrix.scaling_list_8x8[i - 6].write_to(writer)?;
                    }
                }
            }
        } else {
            writer.write_bit(false)?; // seq_scaling_matrix_present_flag
        }
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for PicOrderCntType1 {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_bit(self.delta_pic_order_always_zero_flag)?;
        write_se(writer, self.offset_for_non_ref_pic)?;
        write_se(writer, self.offset_for_top_to_bottom_field)?;
        write_ue(writer, self.offset_for_ref_frame.len().to_u64().unwrap())?;
        self.offset_for_ref_frame
            .iter()
            .try_for_each(|item| write_se(writer, *item))
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for FrameCropping {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        write_ue(writer, self.frame_crop_left_offset)?;
        write_ue(writer, self.frame_crop_right_offset)?;
        write_ue(writer, self.frame_crop_top_offset)?;
        write_ue(writer, self.frame_crop_bottom_offset)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for Sps {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write::<8, u8>(self.profile_idc)?;
        writer.write_bit(self.constraint_set0_flag)?;
        writer.write_bit(self.constraint_set1_flag)?;
        writer.write_bit(self.constraint_set2_flag)?;
        writer.write_bit(self.constraint_set3_flag)?;
        writer.write_bit(self.constraint_set4_flag)?;
        writer.write_bit(self.constraint_set5_flag)?;
        writer.write::<2, u8>(0)?; // reserved_zero_2bits
        writer.write::<8, u8>(self.level_idc)?;
        write_ue(writer, self.seq_parameter_set_id)?;
        if let Some(profile_idc_related) = &self.profile_idc_related {
            profile_idc_related.write_to(writer)?;
        }
        write_ue(writer, self.log2_max_frame_num_minus4)?;
        write_ue(writer, self.pic_order_cnt_type)?;
        if let Some(log2_max_pic_order_cnt_lsb_minus4) = self.log2_max_pic_order_cnt_lsb_minus4 {
            write_ue(writer, log2_max_pic_order_cnt_lsb_minus4)?;
        } else if let Some(pic_order_type1) = &self.pic_order_cnt_type_1 {
            pic_order_type1.write_to(writer)?;
        }
        write_ue(writer, self.max_num_ref_frames)?;
        writer.write_bit(self.gaps_in_frame_num_value_allowed_flag)?;
        write_ue(writer, self.pic_width_in_mbs_minus1)?;
        write_ue(writer, self.pic_height_in_map_units_minus1)?;
        if let Some(mb_adaptive_frame_field_flag) = self.mb_adaptive_frame_field_flag {
            writer.write_bit(false)?; // frame_mbs_only_flag
            writer.write_bit(mb_adaptive_frame_field_flag)?;
        } else {
            writer.write_bit(true)?; // frame_mbs_only_flag
        }
        writer.write_bit(self.direct_8x8_inference_flag)?;
        if let Some(frame_cropping) = &self.frame_cropping {
            writer.write_bit(true)?; // frame_cropping_flag
            frame_cropping.write_to(writer)?;
        } else {
            writer.write_bit(false)?; // frame_cropping_flag
        }
        if let Some(vui) = &self.vui_parameters {
            writer.write_bit(true)?; // vui_parameters_present_flag
            vui.write_to(writer)?;
        } else {
            writer.write_bit(false)?; // vui_parameters_present_flag
        }
        Ok(())
    }
}
