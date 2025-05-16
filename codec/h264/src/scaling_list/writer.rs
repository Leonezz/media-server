use bitstream_io::BitWrite;
use utils::traits::writer::BitwiseWriteTo;

use crate::{errors::H264CodecError, exp_golomb::write_se};

use super::ScalingListRaw;

impl<const C: usize, W: BitWrite> BitwiseWriteTo<W> for ScalingListRaw<C> {
    type Error = H264CodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        self.delta_scale.iter().try_for_each(|item| {
            if let Some(delta) = item {
                write_se(writer, *delta)?;
            }
            Ok::<(), Self::Error>(())
        })?;
        Ok(())
    }
}
