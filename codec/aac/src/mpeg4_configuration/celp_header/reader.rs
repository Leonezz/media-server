use bitstream_io::BitRead;
use utils::traits::reader::BitwiseReadFrom;

use crate::errors::AACCodecError;

use super::{CelpBWSenhHeader, CelpHeader, ExcitationMode, MPEExciationMode};

impl<R: BitRead> BitwiseReadFrom<R> for MPEExciationMode {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let mpe_configuration = reader.read::<5, u8>()?;
        let num_enh_layers = reader.read::<2, u8>()?;
        let bandwidth_scalability_mode = reader.read_bit()?;
        Ok(Self {
            mpe_configuration,
            num_enh_layers,
            bandwidth_scalability_mode,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for CelpHeader {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let excitation_mode = ExcitationMode::from(reader.read_bit()?);
        let sample_rate_mode = reader.read_bit()?;
        let fine_rate_control = reader.read_bit()?;
        let rpe_configuration = if excitation_mode == ExcitationMode::RPE {
            Some(reader.read::<3, u8>()?)
        } else {
            None
        };
        let mpe_exciation_mode = if excitation_mode == ExcitationMode::MPE {
            Some(MPEExciationMode::read_from(reader)?)
        } else {
            None
        };
        Ok(Self {
            excitation_mode,
            sample_rate_mode: sample_rate_mode.into(),
            fine_rate_control: fine_rate_control.into(),
            rpe_configuration,
            mpe_exciation_mode,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for CelpBWSenhHeader {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let bws_configuration = reader.read::<2, u8>()?;
        Ok(Self { bws_configuration })
    }
}
