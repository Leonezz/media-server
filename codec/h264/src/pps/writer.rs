use bitstream_io::BitWrite;
use num::ToPrimitive;
use utils::traits::writer::BitwiseWriteTo;

use crate::{
    errors::H264CodecError,
    exp_golomb::{write_se, write_ue},
};

use super::{
    MoreData, NumSliceGroupsMinus1Positive, Pps, SliceGroupMapType0, SliceGroupMapType2,
    SliceGroupMapType6, SliceGroupMapType345, SliceGroupMapTypeRelated,
};

impl<W: BitWrite> BitwiseWriteTo<W> for SliceGroupMapType0 {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        self.run_length_minus1.iter().try_for_each(|item| {
            write_ue(writer, *item)?;
            Ok::<(), Self::Error>(())
        })?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for SliceGroupMapType2 {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        self.items.iter().try_for_each(|item| {
            write_ue(writer, item.top_left)?;
            write_ue(writer, item.bottom_right)?;
            Ok::<(), Self::Error>(())
        })
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for SliceGroupMapType345 {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_bit(self.slice_group_change_direction_flag)?;
        write_ue(writer, self.slice_group_change_rate_minus1)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for SliceGroupMapType6 {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        write_ue(writer, self.slice_group_id.len().to_u64().unwrap())?; // pic_size_in_map_units_minus1
        assert!(self.bits_cnt > 0);
        self.slice_group_id.iter().try_for_each(|item| {
            writer.write_var(self.bits_cnt, *item)?;
            Ok::<(), Self::Error>(())
        })?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for SliceGroupMapTypeRelated {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        match self {
            SliceGroupMapTypeRelated::Type0(item) => item.write_to(writer),
            SliceGroupMapTypeRelated::Type2(item) => item.write_to(writer),
            SliceGroupMapTypeRelated::Type345(item) => item.write_to(writer),
            SliceGroupMapTypeRelated::Type6(item) => item.write_to(writer),
        }
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for NumSliceGroupsMinus1Positive {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        write_ue(writer, self.slice_group_map_type)?;
        self.slice_group_map_type_related.write_to(writer)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for MoreData {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_bit(self.transform_8x8_flag)?;
        if let Some(pic_scaling_matrix) = &self.pic_scaling_matrix {
            writer.write_bit(true)?; // pic_scaling_matrix_present_flag
            for (i, present) in pic_scaling_matrix
                .seq_scaling_list_present_flag
                .iter()
                .enumerate()
            {
                if !*present {
                    continue;
                }
                if i < 6 {
                    pic_scaling_matrix.scaling_list_4x4[i].write_to(writer)?;
                } else {
                    pic_scaling_matrix.scaling_list_8x8[i - 6].write_to(writer)?;
                }
            }
        } else {
            writer.write_bit(false)?; // pic_scaling_matrix_present_flag
        }
        write_se(writer, self.second_chroma_qp_index_offset)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for Pps {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        write_ue(writer, self.pic_parameter_set_id.to_u64().unwrap())?;
        write_ue(writer, self.seq_parameter_set_id.to_u64().unwrap())?;
        writer.write_bit(self.entropy_coding_mode_flag)?;
        writer.write_bit(self.bottom_field_pic_order_in_frame_present_flag)?;
        write_ue(writer, self.num_slice_groups_minus1)?;
        if let Some(num_slice_groups_minus1_positive) = &self.num_slice_groups_minus1_positive {
            num_slice_groups_minus1_positive.write_to(writer)?;
        }
        write_ue(
            writer,
            self.num_ref_idx_10_default_active_minus1.to_u64().unwrap(),
        )?;
        write_ue(
            writer,
            self.num_ref_idx_11_default_active_minus1.to_u64().unwrap(),
        )?;
        writer.write_bit(self.weighted_pred_flag)?;
        writer.write::<2, u8>(self.weighted_bipred_idc)?;
        write_se(writer, self.pic_init_qp_minus26.to_i64().unwrap())?;
        write_se(writer, self.pic_init_qs_minus26.to_i64().unwrap())?;
        write_se(writer, self.chroma_qp_index_offset.to_i64().unwrap())?;
        writer.write_bit(self.deblocking_filter_control_present_flag)?;
        writer.write_bit(self.constrained_intra_pred_flag)?;
        writer.write_bit(self.redundant_pic_cnt_present_flag)?;
        if let Some(more_data) = &self.more_data {
            more_data.write_to(writer)?;
        }
        Ok(())
    }
}
