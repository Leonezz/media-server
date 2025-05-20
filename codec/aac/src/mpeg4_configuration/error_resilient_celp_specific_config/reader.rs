use bitstream_io::BitRead;
use utils::traits::reader::{BitwiseReadFrom, BitwiseReadReaminingFrom};

use crate::{
    errors::AACCodecError,
    mpeg4_configuration::{
        audio_specific_config::sampling_frequency_index::SamplingFrequencyIndex,
        celp_header::{CelpBWSenhHeader, ExcitationMode, MPEExciationMode},
    },
};

use super::{ErScCelpHeader, ErrorResilientCelpSpecificConfig};

impl<R: BitRead> BitwiseReadReaminingFrom<SamplingFrequencyIndex, R> for ErScCelpHeader {
    type Error = AACCodecError;
    fn read_remaining_from(
        sampling_frequency_index: SamplingFrequencyIndex,
        reader: &mut R,
    ) -> Result<Self, Self::Error> {
        let excitation_mode: ExcitationMode = reader.read_bit()?.into();
        let sample_rate_mode = reader.read_bit()?;
        let fine_rate_control = reader.read_bit()?;
        let silence_compression = reader.read_bit()?;
        let rpe_configuration = if excitation_mode == ExcitationMode::RPE {
            Some(reader.read::<3, u8>()?)
        } else {
            None
        };
        let excitation_mode_mpe = if excitation_mode == ExcitationMode::MPE {
            Some(MPEExciationMode::read_from(reader)?)
        } else {
            None
        };
        Ok(Self {
            excitation_mode,
            sample_rate_mode: sample_rate_mode.into(),
            fine_rate_control: fine_rate_control.into(),
            silence_compression,
            rpe_configuration,
            excitation_mode_mpe,
            sampling_frequency_index,
        })
    }
}

impl<R: BitRead> BitwiseReadReaminingFrom<SamplingFrequencyIndex, R>
    for ErrorResilientCelpSpecificConfig
{
    type Error = AACCodecError;
    fn read_remaining_from(
        sampling_frequency_indec: SamplingFrequencyIndex,
        reader: &mut R,
    ) -> Result<Self, Self::Error> {
        let is_base_layer = reader.read_bit()?;
        let er_sc_celp_header = if is_base_layer {
            Some(ErScCelpHeader::read_remaining_from(
                sampling_frequency_indec,
                reader,
            )?)
        } else {
            None
        };
        let is_bws_layer = if !is_base_layer {
            Some(reader.read_bit()?)
        } else {
            None
        };
        let celp_bw_senh_header = if let Some(is_bws_layer) = is_bws_layer
            && is_bws_layer
        {
            Some(CelpBWSenhHeader::read_from(reader)?)
        } else {
            None
        };
        let celp_brs_id = if let Some(is_bws_layer) = is_bws_layer
            && !is_bws_layer
        {
            Some(reader.read::<2, u8>()?)
        } else {
            None
        };
        Ok(Self {
            is_base_layer,
            er_sc_celp_header,
            is_bws_layer,
            celp_bw_senh_header,
            celp_brs_id,
        })
    }
}
