use bitstream_io::BitRead;
use num::ToPrimitive;
use utils::traits::reader::{BitwiseReadFrom, BitwiseReadReaminingFrom};

use crate::{
    errors::H264CodecError,
    exp_golomb::{read_se, read_ue},
    rbsp::RbspReadExt,
    scaling_list::{ScalingListRaw, SeqScalingMatrix},
    sps::chroma_format_idc::ChromaFormatIdc,
};

use super::{
    MoreData, NumSliceGroupsMinus1Positive, Pps, SliceGroupMapType0, SliceGroupMapType2,
    SliceGroupMapType6, SliceGroupMapType345, SliceGroupMapTypeRelated, SliceGroupMaptype2Item,
};

impl<R: BitRead> BitwiseReadReaminingFrom<usize, R> for SliceGroupMapType0 {
    type Error = H264CodecError;
    fn read_remaining_from(header: usize, reader: &mut R) -> Result<Self, Self::Error> {
        let mut run_length_minus1 = vec![0; header];
        run_length_minus1.iter_mut().try_for_each(|item| {
            *item = read_ue(reader)?;
            Ok::<(), Self::Error>(())
        })?;
        Ok(Self { run_length_minus1 })
    }
}

impl<R: BitRead> BitwiseReadReaminingFrom<usize, R> for SliceGroupMapType2 {
    type Error = H264CodecError;
    fn read_remaining_from(header: usize, reader: &mut R) -> Result<Self, Self::Error> {
        let mut items = vec![
            SliceGroupMaptype2Item {
                top_left: 0,
                bottom_right: 0,
            };
            header
        ];
        items.iter_mut().try_for_each(|item| {
            item.top_left = read_ue(reader)?;
            item.bottom_right = read_ue(reader)?;
            Ok::<(), Self::Error>(())
        })?;
        Ok(Self { items })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for SliceGroupMapType345 {
    type Error = H264CodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let slice_group_change_direction_flag = reader.read_bit()?;
        let slice_group_change_rate_minus1 = read_ue(reader)?;
        Ok(Self {
            slice_group_change_direction_flag,
            slice_group_change_rate_minus1,
        })
    }
}

impl<R: BitRead> BitwiseReadReaminingFrom<usize, R> for SliceGroupMapType6 {
    type Error = H264CodecError;
    fn read_remaining_from(header: usize, reader: &mut R) -> Result<Self, Self::Error> {
        let pic_size_in_map_units_minus1 = read_ue(reader)?;
        let bits_cnt = header
            .checked_add(1)
            .and_then(|v| v.to_f64())
            .and_then(|v| v.log2().ceil().to_u32())
            .unwrap();
        let mut slice_group_id = vec![
            0;
            pic_size_in_map_units_minus1
                .checked_add(1)
                .and_then(|v| v.to_usize())
                .unwrap()
        ];
        slice_group_id.iter_mut().try_for_each(|item| {
            *item = reader.read_var(bits_cnt)?;
            Ok::<(), Self::Error>(())
        })?;
        Ok(Self {
            pic_size_in_map_units_minus1,
            bits_cnt,
            slice_group_id,
        })
    }
}

impl<R: BitRead> BitwiseReadReaminingFrom<(usize, u8), R> for SliceGroupMapTypeRelated {
    type Error = H264CodecError;
    fn read_remaining_from(header: (usize, u8), reader: &mut R) -> Result<Self, Self::Error> {
        let (num_slice_groups_minus1, slice_group_map_type) = header;
        let slice_group_map_type_related = match slice_group_map_type {
            0 => SliceGroupMapTypeRelated::Type0(SliceGroupMapType0::read_remaining_from(
                num_slice_groups_minus1,
                reader,
            )?),
            2 => SliceGroupMapTypeRelated::Type2(SliceGroupMapType2::read_remaining_from(
                num_slice_groups_minus1,
                reader,
            )?),
            3..=5 => SliceGroupMapTypeRelated::Type345(SliceGroupMapType345::read_from(reader)?),
            6 => SliceGroupMapTypeRelated::Type6(SliceGroupMapType6::read_remaining_from(
                num_slice_groups_minus1,
                reader,
            )?),
            _ => unreachable!(),
        };
        Ok(slice_group_map_type_related)
    }
}

impl<R: BitRead> BitwiseReadReaminingFrom<usize, R> for NumSliceGroupsMinus1Positive {
    type Error = H264CodecError;
    fn read_remaining_from(header: usize, reader: &mut R) -> Result<Self, Self::Error> {
        let slice_group_map_type = read_ue(reader)?.to_u8().unwrap();
        let slice_group_map_type_related =
            SliceGroupMapTypeRelated::read_remaining_from((header, slice_group_map_type), reader)?;
        Ok(Self {
            slice_group_map_type,
            slice_group_map_type_related,
        })
    }
}

impl<R: BitRead> BitwiseReadReaminingFrom<ChromaFormatIdc, R> for MoreData {
    type Error = H264CodecError;
    fn read_remaining_from(
        chroma_format_idc: ChromaFormatIdc,
        reader: &mut R,
    ) -> Result<Self, Self::Error> {
        let transform_8x8_flag = reader.read_bit()?;
        let pic_scaling_matrix_present_flag = reader.read_bit()?;
        let pic_scaling_matrix = if pic_scaling_matrix_present_flag {
            let cnt = if transform_8x8_flag {
                if !matches!(chroma_format_idc, ChromaFormatIdc::Chroma444) {
                    2_usize
                } else {
                    6_usize
                }
            } else {
                0_usize
            }
            .checked_add(6)
            .unwrap();
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
                            reader,
                            &mut use_default_scaling_matrix_4x4_flag[i],
                        )?;
                    } else {
                        scaling_list_8x8[i - 6] = ScalingListRaw::<64>::new(
                            reader,
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
        let second_chroma_qp_index_offset = read_se(reader)?;
        Ok(Self {
            transform_8x8_flag,
            pic_scaling_matrix_present_flag,
            pic_scaling_matrix,
            second_chroma_qp_index_offset,
        })
    }
}

impl<R: BitRead + RbspReadExt<Error = H264CodecError>> BitwiseReadReaminingFrom<ChromaFormatIdc, R>
    for Pps
{
    type Error = H264CodecError;
    fn read_remaining_from(
        chroma_format_idc: ChromaFormatIdc,
        reader: &mut R,
    ) -> Result<Self, Self::Error> {
        let pic_parameter_set_id = read_ue(reader)?.to_u8().unwrap();
        let seq_parameter_set_id = read_ue(reader)?.to_u8().unwrap();
        let entropy_coding_mode_flag = reader.read_bit()?;
        let bottom_field_pic_order_in_frame_present_flag = reader.read_bit()?;
        let num_slice_groups_minus1 = read_ue(reader)?;
        let num_slice_groups_minus1_positive = if num_slice_groups_minus1 > 0 {
            Some(NumSliceGroupsMinus1Positive::read_remaining_from(
                num_slice_groups_minus1.to_usize().unwrap(),
                reader,
            )?)
        } else {
            None
        };
        let num_ref_idx_10_default_active_minus1 = read_ue(reader)?.to_u8().unwrap();
        let num_ref_idx_11_default_active_minus1 = read_ue(reader)?.to_u8().unwrap();
        let weighted_pred_flag = reader.read_bit()?;
        let weighted_bipred_idc = reader.read::<2, u8>()?;
        let pic_init_qp_minus26 = read_se(reader)?.to_i8().unwrap();
        let pic_init_qs_minus26 = read_se(reader)?.to_i8().unwrap();
        let chroma_qp_index_offset = read_se(reader)?.to_i8().unwrap();
        let deblocking_filter_control_present_flag = reader.read_bit()?;
        let constrained_intra_pred_flag = reader.read_bit()?;
        let redundant_pic_cnt_present_flag = reader.read_bit()?;
        let more_data = if reader.more_rbsp_data()? {
            Some(MoreData::read_remaining_from(chroma_format_idc, reader)?)
        } else {
            None
        };
        Ok(Self {
            pic_parameter_set_id,
            seq_parameter_set_id,
            entropy_coding_mode_flag,
            bottom_field_pic_order_in_frame_present_flag,
            num_slice_groups_minus1,
            num_slice_groups_minus1_positive,
            num_ref_idx_10_default_active_minus1,
            num_ref_idx_11_default_active_minus1,
            weighted_pred_flag,
            weighted_bipred_idc,
            pic_init_qp_minus26,
            pic_init_qs_minus26,
            chroma_qp_index_offset,
            deblocking_filter_control_present_flag,
            constrained_intra_pred_flag,
            redundant_pic_cnt_present_flag,
            more_data,
        })
    }
}
