use bitstream_io::BitRead;
use utils::traits::reader::BitwiseReadFrom;

use crate::{
    errors::AACCodecError,
    mpeg4_configuration::celp_header::{CelpBWSenhHeader, CelpHeader},
};

use super::CelpSpecificConfig;

impl<R: BitRead> BitwiseReadFrom<R> for CelpSpecificConfig {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let is_base_layer = reader.read_bit()?;
        let celp_header = if is_base_layer {
            Some(CelpHeader::read_from(reader)?)
        } else {
            None
        };
        let is_bws_layer = if !is_base_layer {
            Some(reader.read_bit()?)
        } else {
            None
        };
        let celp_bwsenh_header = if let Some(is_bws_layer) = is_bws_layer
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
            celp_header,
            is_bws_layer,
            celp_bwsenh_header,
            celp_brs_id,
        })
    }
}
