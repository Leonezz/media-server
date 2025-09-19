use bitstream_io::BitRead;
use num::ToPrimitive;
use utils::traits::reader::BitwiseReadFrom;

use crate::{errors::H264CodecError, exp_golomb::read_ue};

use super::{AuxFormatIdcRelated, SpsExt};

impl<R: BitRead> BitwiseReadFrom<R> for AuxFormatIdcRelated {
    type Error = H264CodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let bit_depth_aux_minus8 = read_ue(reader)?.to_u8().unwrap();
        let alpha_incr_flag = reader.read_bit()?;
        let bit_depth = bit_depth_aux_minus8
            .to_u32()
            .and_then(|v| v.checked_add(9))
            .unwrap();
        let alpha_opaque_value = reader.read_var::<u16>(bit_depth)?;
        let alpha_transparent_value = reader.read_var(bit_depth)?;
        Ok(Self {
            bit_depth_aux_minus8,
            alpha_incr_flag,
            alpha_opaque_value,
            alpha_transparent_value,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for SpsExt {
    type Error = H264CodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let seq_parameter_set_id = read_ue(reader)?;
        let aux_format_idc = read_ue(reader)?.to_u8().unwrap();
        let aux_format_idc_related = if aux_format_idc != 0 {
            Some(AuxFormatIdcRelated::read_from(reader)?)
        } else {
            None
        };
        let additional_extension_flag = reader.read_bit()?;
        Ok(Self {
            seq_parameter_set_id,
            aux_format_idc,
            aux_format_idc_related,
            additional_extension_flag,
        })
    }
}
