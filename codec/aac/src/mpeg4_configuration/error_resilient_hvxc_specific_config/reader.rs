use bitstream_io::BitRead;
use utils::traits::reader::BitwiseReadFrom;

use crate::{
    errors::AACCodecError,
    mpeg4_configuration::hvxc_specific_config::{HVXCrateMode, HVXCvarMode},
};

use super::{ErHvxcConfig, ErrorResilientHvxcSpecificConfig};

impl<R: BitRead> BitwiseReadFrom<R> for ErHvxcConfig {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let hvxc_var_mode: HVXCvarMode = reader.read_bit()?.into();
        let hvxc_rate_mode: HVXCrateMode = reader.read::<2, u8>()?.try_into()?;
        let extension_flag = reader.read_bit()?;
        let var_scalable_flag = if extension_flag {
            Some(reader.read_bit()?)
        } else {
            None
        };
        Ok(Self {
            hvxc_var_mode,
            hvxc_rate_mode,
            extension_flag,
            var_scalable_flag,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for ErrorResilientHvxcSpecificConfig {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let is_base_layer = reader.read_bit()?;
        let config = if is_base_layer {
            Some(ErHvxcConfig::read_from(reader)?)
        } else {
            None
        };
        Ok(Self {
            is_base_layer,
            config,
        })
    }
}
