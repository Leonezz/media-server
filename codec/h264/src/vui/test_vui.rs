#[cfg(test)]
mod test {
    use utils::traits::{reader::BitwiseReadFrom, writer::BitwiseWriteTo};

    use crate::{
        rbsp::{rbsp_extract, rbsp_to_sodb},
        vui::{
            AspectRatioInfo, BitstreamRestriction, ColourDescription, TimingInfo, VideoSignalType,
            VuiParameters,
        },
    };

    #[test]
    fn test_vui() {
        let vui = VuiParameters {
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
        };

        let mut bytes = Vec::new();
        let mut writer = bitstream_io::BitWriter::endian(&mut bytes, bitstream_io::BigEndian);
        vui.write_to(&mut writer).unwrap();
        let bytes = rbsp_extract(&rbsp_to_sodb(&bytes));
        let mut reader = bitstream_io::BitReader::endian(&bytes[..], bitstream_io::BigEndian);
        let vui_parsed = VuiParameters::read_from(&mut reader).unwrap();
        assert_eq!(
            vui_parsed.timing_info.as_ref().unwrap().num_units_in_tick,
            1
        );
        assert_eq!(vui_parsed.timing_info.as_ref().unwrap().time_scale, 60);
    }
}
