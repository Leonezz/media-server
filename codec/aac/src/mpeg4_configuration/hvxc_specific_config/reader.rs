use bitstream_io::BitRead;
use utils::traits::reader::BitwiseReadFrom;

use crate::errors::AACCodecError;

use super::{HVXCConfig, HvxcSpecificConfig};

impl<R: BitRead> BitwiseReadFrom<R> for HVXCConfig {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let hvxc_var_mode = reader.read_bit()?.into();
        let hvxc_rate_mode = reader.read::<2, u8>()?;
        let extension_flag = reader.read_bit()?;
        if extension_flag {
            panic!("the spec says TO BE defined in MPEG-4 Version 2");
        }
        Ok(Self {
            hvxc_var_mode,
            hvxc_rate_mode: hvxc_rate_mode.try_into()?,
            extension_flag,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for HvxcSpecificConfig {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let is_base_layer = reader.read_bit()?;
        let config = if is_base_layer {
            Some(HVXCConfig::read_from(reader)?)
        } else {
            None
        };
        Ok(Self {
            is_base_layer,
            config,
        })
    }
}
