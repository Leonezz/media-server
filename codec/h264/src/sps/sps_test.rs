#[cfg(test)]
mod test {
    use bitstream_io::BitRead;
    use utils::traits::reader::BitwiseReadFrom;

    use crate::{
        nalu::NalUnit,
        rbsp::rbsp_extract,
        sps::{FrameCropping, ProfileIdcRelated, Sps},
        vui::{
            AspectRatioInfo, BitstreamRestriction, ColourDescription, TimingInfo, VideoSignalType,
            VuiParameters,
        },
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
            vui_parameters_present_flag: true,
            vui_parameters: Some(VuiParameters {
                aspect_ratio_info_present_flag: true,
                aspect_ratio_info: Some(AspectRatioInfo {
                    aspect_ratio_idc: crate::vui::AspectRatioIdc::Square,
                    aspect_ratio_info_extended_sar: None,
                }),
                overscan_info_present_flag: false,
                overscan_appropriate_flag: None,
                video_signal_type_present_flag: true,
                video_signal_type: Some(VideoSignalType {
                    video_format: crate::vui::VideoFormat::Unspecified,
                    video_full_range_flag: false,
                    colour_description_present_flag: true,
                    colour_description: Some(ColourDescription {
                        colour_primaries: 1,
                        transfer_characteristics: 1,
                        matrix_coefficients: 1,
                    }),
                }),
                chroma_loc_info_present_flag: false,
                chroma_loc_info: None,
                timing_info_present_flag: true,
                timing_info: Some(TimingInfo {
                    num_units_in_tick: 1,
                    time_scale: 60,
                    fixed_frame_rate_flag: true,
                }),
                nal_hrd_parameters_present_flag: false,
                nal_hrd_parameters: None,
                vcl_hrd_parameters_present_flag: false,
                vcl_hrd_parameters: None,
                pic_struct_present_flag: false,
                bitstream_restriction_flag: true,
                bitstream_restriction: Some(BitstreamRestriction {
                    motion_vectors_over_pic_boundaries_flag: true,
                    max_bytes_per_pic_denom: 0,
                    max_bits_per_mb_denom: 0,
                    log2_max_mv_length_horizontal: 11,
                    log2_max_mv_length_vertical: 11,
                    max_num_reorder_frames: 2,
                    max_dec_frame_buffering: 4,
                }),
                low_delay_hrd_flag: None,
            }),
        };
        let nalu: NalUnit = (&sps).into();
        let bytes = rbsp_extract(&nalu.body[..]);
        let mut reader = bitstream_io::BitReader::endian(&bytes[..], bitstream_io::BigEndian);
        let sps_parsed = Sps::read_from(reader.by_ref()).unwrap();
        assert!(reader.read_bit().unwrap());
        assert_eq!(sps_parsed.seq_parameter_set_id, 0);
    }
}
