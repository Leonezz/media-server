#[cfg(test)]
mod test {
    use bitstream_io::BitRead;
    use utils::traits::reader::BitwiseReadFrom;

    use crate::{
        nalu::NalUnit,
        rbsp::rbsp_extract,
        sps::{FrameCropping, ProfileIdcRelated, Sps},
    };

    #[test]
    fn test_sps_nalu() {
        let sps = Sps {
            profile_idc: 100,
            constraint_set0_flag: false,
            constraint_set1_flag: false,
            constraint_set2_flag: false,
            constraint_set3_flag: false,
            constraint_set4_flag: false,
            constraint_set5_flag: false,
            reserved_zero_2bits: 0,
            level_idc: 31,
            seq_parameter_set_id: 0,
            profile_idc_related: Some(ProfileIdcRelated {
                chroma_format_idc: crate::sps::chroma_format_idc::ChromaFormatIdc::Chroma420,
                separate_colour_plane_flag: None,
                bit_depth_luma_minus8: 0,
                bit_depth_chroma_minus8: 0,
                qpprime_y_zero_transform_bypass_flag: false,
                seq_scaling_matrix_present_flag: false,
                seq_scaling_matrix: None,
            }),
            log2_max_frame_num_minus4: 1,
            pic_order_cnt_type: 0,
            log2_max_pic_order_cnt_lsb_minus4: Some(2),
            pic_order_cnt_type_1: None,
            max_num_ref_frames: 1,
            gaps_in_frame_num_value_allowed_flag: false,
            pic_width_in_mbs_minus1: 53,
            pic_height_in_map_units_minus1: 29,
            frame_mbs_only_flag: true,
            mb_adaptive_frame_field_flag: Some(true),
            direct_8x8_inference_flag: true,
            frame_cropping_flag: true,
            frame_cropping: Some(FrameCropping {
                frame_crop_left_offset: 5,
                frame_crop_right_offset: 0,
                frame_crop_top_offset: 0,
                frame_crop_bottom_offset: 1,
            }),
            vui_parameters_present_flag: false,
            vui_parameters: None,
        };
        let nalu: NalUnit = (&sps).into();
        let bytes = rbsp_extract(&nalu.body[..]);
        let mut reader = bitstream_io::BitReader::endian(&bytes[..], bitstream_io::BigEndian);
        let sps_parsed = Sps::read_from(reader.by_ref()).unwrap();
        assert!(reader.read_bit().unwrap());
        assert_eq!(sps_parsed.seq_parameter_set_id, 0);
    }
}
