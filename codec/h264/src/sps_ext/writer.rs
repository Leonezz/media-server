use bitstream_io::BitWrite;
use num::ToPrimitive;
use utils::traits::writer::BitwiseWriteTo;

use crate::{errors::H264CodecError, exp_golomb::write_ue};

use super::{AuxFormatIdcRelated, SpsExt};

impl<W: BitWrite> BitwiseWriteTo<W> for AuxFormatIdcRelated {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        write_ue(writer, self.bit_depth_aux_minus8)?;
        writer.write_bit(self.alpha_incr_flag)?;
        let bit_depth = self
            .bit_depth_aux_minus8
            .to_u32()
            .and_then(|v| v.checked_add(9))
            .unwrap();
        writer.write_var(bit_depth, self.alpha_opaque_value)?;
        writer.write_var(bit_depth, self.alpha_transparent_value)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for SpsExt {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        write_ue(writer, self.seq_parameter_set_id)?;
        write_ue(writer, self.aux_format_idc)?;
        if let Some(aux_format_related) = &self.aux_format_idc_related {
            aux_format_related.write_to(writer)?;
        }
        writer.write_bit(self.additional_extension_flag)?;
        Ok(())
    }
}
