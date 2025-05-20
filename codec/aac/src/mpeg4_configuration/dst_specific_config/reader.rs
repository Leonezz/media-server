use bitstream_io::BitRead;
use utils::traits::reader::BitwiseReadFrom;

use crate::errors::AACCodecError;

use super::DSTSpecificConfig;

impl<R: BitRead> BitwiseReadFrom<R> for DSTSpecificConfig {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let dsddst_coded = reader.read_bit()?;
        let n_channels = reader.read::<14, u16>()?;
        let reserved = reader.read_bit()?;
        Ok(Self {
            dsddst_coded,
            n_channels,
            reserved,
        })
    }
}
