use bitstream_io::BitRead;
use num::ToPrimitive;
use utils::traits::reader::BitwiseReadFrom;

use crate::{errors::H264CodecError, exp_golomb::read_ue};

use super::{
    AspectRatioIdc, AspectRatioInfo, AspectRatioInfoExtendedSAR, BitstreamRestriction,
    ChromaLocInfo, ColourDescription, TimingInfo, VideoFormat, VideoSignalType, VuiParameters,
    hrd_parameters::{HrdParameters, SchedSel},
};

impl<R: BitRead> BitwiseReadFrom<R> for SchedSel {
    type Error = H264CodecError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let bit_rate_value_minus1 = read_ue(&mut reader)?;
        let cpb_size_value_minus1 = read_ue(&mut reader)?;
        let cbr_flag = reader.read_bit()?;
        Ok(Self {
            bit_rate_value_minus1,
            cpb_size_value_minus1,
            cbr_flag,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for HrdParameters {
    type Error = H264CodecError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let cpb_cnt_minus1 = read_ue(&mut reader)?;
        let cpb_cnt_minus1 = cpb_cnt_minus1.to_u8().unwrap();

        let bit_rate_scale = reader.read::<4, u8>()?;
        let cpb_size_scale = reader.read::<4, u8>()?;
        let mut sched_sels = vec![];
        for _ in 0..=cpb_cnt_minus1 {
            let item = SchedSel::read_from(&mut reader)?;
            sched_sels.push(item);
        }
        let initial_cpb_removal_delay_length_minus1 = reader.read::<5, u8>()?;
        let cpb_removal_delay_length_minus1 = reader.read::<5, u8>()?;
        let dpb_output_delay_length_minus1 = reader.read::<5, u8>()?;
        let time_offset_length = reader.read::<5, u8>()?;
        Ok(Self {
            cpb_cnt_minus1,
            bit_rate_scale,
            cpb_size_scale,
            sched_sels,
            initial_cpb_removal_delay_length_minus1,
            cpb_removal_delay_length_minus1,
            dpb_output_delay_length_minus1,
            time_offset_length,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for AspectRatioInfoExtendedSAR {
    type Error = H264CodecError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let sar_width = reader.read::<16, u16>()?;
        let sar_height = reader.read::<16, u16>()?;
        Ok(Self {
            sar_width,
            sar_height,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for AspectRatioInfo {
    type Error = H264CodecError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let byte = reader.read::<8, u8>()?;
        let aspect_ratio_idc = AspectRatioIdc::from(byte);
        let aspect_ratio_info_extended_sar =
            if matches!(aspect_ratio_idc, AspectRatioIdc::ExtendedSAR) {
                Some(AspectRatioInfoExtendedSAR::read_from(&mut reader)?)
            } else {
                None
            };
        Ok(Self {
            aspect_ratio_idc,
            aspect_ratio_info_extended_sar,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for ColourDescription {
    type Error = H264CodecError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let colour_primaries = reader.read::<8, u8>()?;
        let transfer_characteristics = reader.read::<8, u8>()?;
        let matrix_coefficients = reader.read::<8, u8>()?;
        Ok(Self {
            colour_primaries,
            transfer_characteristics,
            matrix_coefficients,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for VideoSignalType {
    type Error = H264CodecError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let byte = reader.read::<3, u8>()?;
        let video_format: VideoFormat = byte.try_into()?;
        let video_full_range_flag = reader.read_bit()?;
        let colour_description_present_flag = reader.read_bit()?;
        let colour_description = if colour_description_present_flag {
            Some(ColourDescription::read_from(&mut reader)?)
        } else {
            None
        };
        Ok(Self {
            video_format,
            video_full_range_flag,
            colour_description_present_flag,
            colour_description,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for ChromaLocInfo {
    type Error = H264CodecError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let chroma_sample_loc_type_top_field = read_ue(&mut reader)?.to_u8().unwrap();
        let chroma_sample_loc_type_bottom_field = read_ue(&mut reader)?.to_u8().unwrap();
        Ok(Self {
            chroma_sample_loc_type_top_field,
            chroma_sample_loc_type_bottom_field,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for TimingInfo {
    type Error = H264CodecError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let num_units_in_tick = reader.read::<32, u32>()?;
        let time_scale = reader.read::<32, u32>()?;
        let fixed_frame_rate_flag = reader.read_bit()?;
        Ok(Self {
            num_units_in_tick,
            time_scale,
            fixed_frame_rate_flag,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for BitstreamRestriction {
    type Error = H264CodecError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let motion_vectors_over_pic_boundaries_flag = reader.read_bit()?;
        let max_bytes_per_pic_denom = read_ue(&mut reader)?.to_u8().unwrap();
        let max_bits_per_mb_denom = read_ue(&mut reader)?.to_u8().unwrap();
        let log2_max_mv_length_horizontal = read_ue(&mut reader)?.to_u8().unwrap();
        let log2_max_mv_length_vertical = read_ue(&mut reader)?.to_u8().unwrap();
        let max_num_reorder_frames = read_ue(&mut reader)?;
        let max_dec_frame_buffering = read_ue(&mut reader)?;
        Ok(Self {
            motion_vectors_over_pic_boundaries_flag,
            max_bytes_per_pic_denom,
            max_bits_per_mb_denom,
            log2_max_mv_length_horizontal,
            log2_max_mv_length_vertical,
            max_num_reorder_frames,
            max_dec_frame_buffering,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for VuiParameters {
    type Error = H264CodecError;
    fn read_from(mut reader: R) -> Result<Self, Self::Error> {
        let aspect_ratio_info_present_flag = reader.read_bit()?;
        let aspect_ratio_info = if aspect_ratio_info_present_flag {
            Some(AspectRatioInfo::read_from(&mut reader)?)
        } else {
            None
        };
        let overscan_info_present_flag = reader.read_bit()?;
        let overscan_appropriate_flag = if overscan_info_present_flag {
            Some(reader.read_bit()?)
        } else {
            None
        };
        let video_signal_type_present_flag = reader.read_bit()?;
        let video_signal_type = if video_signal_type_present_flag {
            Some(VideoSignalType::read_from(&mut reader)?)
        } else {
            None
        };
        let chroma_loc_info_present_flag = reader.read_bit()?;
        let chroma_loc_info = if chroma_loc_info_present_flag {
            Some(ChromaLocInfo::read_from(&mut reader)?)
        } else {
            None
        };
        let timing_info_present_flag = reader.read_bit()?;
        let timing_info = if timing_info_present_flag {
            Some(TimingInfo::read_from(&mut reader)?)
        } else {
            None
        };
        let nal_hrd_parameters_present_flag = reader.read_bit()?;
        let nal_hrd_parameters = if nal_hrd_parameters_present_flag {
            Some(HrdParameters::read_from(&mut reader)?)
        } else {
            None
        };
        let vcl_hrd_parameters_present_flag = reader.read_bit()?;
        let vcl_hrd_parameters = if vcl_hrd_parameters_present_flag {
            Some(HrdParameters::read_from(&mut reader)?)
        } else {
            None
        };
        let low_delay_hrd_flag =
            if nal_hrd_parameters_present_flag || vcl_hrd_parameters_present_flag {
                Some(reader.read_bit()?)
            } else {
                None
            };
        let pic_struct_present_flag = reader.read_bit()?;
        let bitstream_restriction_flag = reader.read_bit()?;
        let bitstream_restriction = if bitstream_restriction_flag {
            Some(BitstreamRestriction::read_from(&mut reader)?)
        } else {
            None
        };
        Ok(Self {
            aspect_ratio_info_present_flag,
            aspect_ratio_info,
            overscan_info_present_flag,
            overscan_appropriate_flag,
            video_signal_type_present_flag,
            video_signal_type,
            chroma_loc_info_present_flag,
            chroma_loc_info,
            timing_info_present_flag,
            timing_info,
            nal_hrd_parameters_present_flag,
            nal_hrd_parameters,
            vcl_hrd_parameters_present_flag,
            vcl_hrd_parameters,
            low_delay_hrd_flag,
            pic_struct_present_flag,
            bitstream_restriction_flag,
            bitstream_restriction,
        })
    }
}
