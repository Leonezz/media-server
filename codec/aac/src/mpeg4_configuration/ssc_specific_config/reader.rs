use bitstream_io::BitRead;
use utils::traits::reader::BitwiseReadReaminingFrom;

use crate::errors::AACCodecError;

use super::SSCSpecificConfig;

impl<R: BitRead> BitwiseReadReaminingFrom<u8, R> for SSCSpecificConfig {
    type Error = AACCodecError;
    fn read_remaining_from(channel_configuration: u8, reader: &mut R) -> Result<Self, Self::Error> {
        let decoder_level = reader.read::<2, u8>()?;
        let update_rate = reader.read::<4, u8>()?;
        let synthesis_method = reader.read::<2, u8>()?;
        let (mode_ext, reserved) = if channel_configuration != 1 {
            let mode_ext = reader.read::<2, u8>()?;
            let reserved = if channel_configuration == 2 && mode_ext == 1 {
                Some(reader.read::<2, u8>()?)
            } else {
                None
            };
            (Some(mode_ext), reserved)
        } else {
            (None, None)
        };
        Ok(Self {
            decoder_level,
            update_rate,
            synthesis_method,
            mode_ext,
            reserved,
        })
    }
}
