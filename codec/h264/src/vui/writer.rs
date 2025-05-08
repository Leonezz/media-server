use bitstream_io::BitWrite;
use num::ToPrimitive;
use utils::traits::writer::BitwiseWriteTo;

use crate::{errors::H264CodecError, exp_golomb::write_ue};

use super::{
    AspectRatioInfo, AspectRatioInfoExtendedSAR, BitstreamRestriction, ChromaLocInfo,
    ColourDescription, TimingInfo, VideoSignalType, VuiParameters,
    hrd_parameters::{HrdParameters, SchedSel},
};

impl<W: BitWrite> BitwiseWriteTo<W> for SchedSel {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        write_ue(writer, self.bit_rate_value_minus1)?;
        write_ue(writer, self.cpb_size_value_minus1)?;
        writer.write_bit(self.cbr_flag)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for HrdParameters {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let cpb_cnt_minus1 = self.sched_sels.len().to_u64().unwrap();
        write_ue(writer, cpb_cnt_minus1)?;
        writer.write::<4, _>(self.bit_rate_scale)?;
        writer.write::<4, _>(self.cpb_size_scale)?;
        self.sched_sels
            .iter()
            .try_for_each(|item| item.write_to(writer))?;
        writer.write::<5, _>(self.initial_cpb_removal_delay_length_minus1)?;
        writer.write::<5, _>(self.cpb_removal_delay_length_minus1)?;
        writer.write::<5, _>(self.dpb_output_delay_length_minus1)?;
        writer.write::<5, _>(self.time_offset_length)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for AspectRatioInfoExtendedSAR {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write::<16, _>(self.sar_width)?;
        writer.write::<16, _>(self.sar_height)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for AspectRatioInfo {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let byte: u8 = self.aspect_ratio_idc.into();
        writer.write::<8, _>(byte)?;
        if let Some(sar) = &self.aspect_ratio_info_extended_sar {
            sar.write_to(writer)?;
        }
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for ColourDescription {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write::<8, u8>(self.colour_primaries)?;
        writer.write::<8, u8>(self.transfer_characteristics)?;
        writer.write::<8, u8>(self.matrix_coefficients)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for VideoSignalType {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let byte: u8 = self.video_format.into();
        writer.write::<3, u8>(byte)?;
        writer.write_bit(self.video_full_range_flag)?;
        // writer.write_bit(self.colour_description_present_flag)?;
        if let Some(dec) = &self.colour_description {
            writer.write_bit(true)?;
            dec.write_to(writer)?;
        } else {
            writer.write_bit(false)?;
        }
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for ChromaLocInfo {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        write_ue(writer, self.chroma_sample_loc_type_top_field)?;
        write_ue(writer, self.chroma_sample_loc_type_bottom_field)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for TimingInfo {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write::<32, u32>(self.num_units_in_tick)?;
        writer.write::<32, u32>(self.time_scale)?;
        writer.write_bit(self.fixed_frame_rate_flag)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for BitstreamRestriction {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_bit(self.motion_vectors_over_pic_boundaries_flag)?;
        write_ue(writer, self.max_bytes_per_pic_denom)?;
        write_ue(writer, self.max_bits_per_mb_denom)?;
        write_ue(writer, self.log2_max_mv_length_horizontal)?;
        write_ue(writer, self.log2_max_mv_length_vertical)?;
        write_ue(writer, self.max_num_reorder_frames)?;
        write_ue(writer, self.max_dec_frame_buffering)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for VuiParameters {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        if let Some(aspect_ratio_info) = &self.aspect_ratio_info {
            writer.write_bit(true)?;
            aspect_ratio_info.write_to(writer)?;
        } else {
            writer.write_bit(false)?;
        }
        if let Some(overscan) = self.overscan_appropriate_flag {
            writer.write_bit(true)?;
            writer.write_bit(overscan)?;
        } else {
            writer.write_bit(false)?;
        }
        if let Some(video_signal) = &self.video_signal_type {
            writer.write_bit(true)?;
            video_signal.write_to(writer)?;
        } else {
            writer.write_bit(false)?;
        }
        if let Some(chroma_loc_info) = &self.chroma_loc_info {
            writer.write_bit(true)?;
            chroma_loc_info.write_to(writer)?;
        } else {
            writer.write_bit(false)?;
        }
        if let Some(timing_info) = &self.timing_info {
            writer.write_bit(true)?;
            timing_info.write_to(writer)?;
        } else {
            writer.write_bit(false)?;
        }
        if let Some(nal_hrd) = &self.nal_hrd_parameters {
            writer.write_bit(true)?;
            nal_hrd.write_to(writer)?;
        } else {
            writer.write_bit(false)?;
        }
        if let Some(vcl_hrd) = &self.vcl_hrd_parameters {
            writer.write_bit(true)?;
            vcl_hrd.write_to(writer)?;
        } else {
            writer.write_bit(false)?;
        }
        if let Some(low_delay_hrd) = self.low_delay_hrd_flag {
            writer.write_bit(low_delay_hrd)?;
        }
        writer.write_bit(self.pic_struct_present_flag)?;
        if let Some(bitstream_restriction) = &self.bitstream_restriction {
            writer.write_bit(true)?;
            bitstream_restriction.write_to(writer)?;
        } else {
            writer.write_bit(false)?;
        }
        Ok(())
    }
}
