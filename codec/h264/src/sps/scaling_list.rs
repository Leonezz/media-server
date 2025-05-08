use bitstream_io::BitRead;

use crate::{
    errors::{H264CodecError, H264CodecResult},
    exp_golomb::read_se,
};

#[derive(Debug, Clone, Copy)]
pub struct ScalingListRaw<const C: usize> {
    pub(crate) delta_scale: [Option<i64>; C], // for write only
    pub scale: [i64; C],
}

impl<const C: usize> Default for ScalingListRaw<C> {
    fn default() -> Self {
        Self {
            scale: [0; C],
            delta_scale: [None; C],
        }
    }
}

impl<const C: usize> ScalingListRaw<C> {
    pub fn new<R: BitRead>(
        mut reader: R,
        use_default_scaling_matrix_flag: &mut bool,
    ) -> H264CodecResult<Self> {
        let mut last_scale: i64 = 8;
        let mut next_scale: i64 = 8;
        let mut scale = [0; C];
        let mut delta_scale = [None; C];
        scale.iter_mut().enumerate().try_for_each(|(i, item)| {
            if next_scale != 0 {
                let delta = read_se(&mut reader)?;
                delta_scale[i] = Some(delta);
                next_scale = last_scale
                    .checked_add(delta)
                    .and_then(|v| v.checked_add(256))
                    .and_then(|v| v.checked_rem(256))
                    .unwrap();
                *use_default_scaling_matrix_flag = i == 0 && next_scale == 0;
            }
            *item = if next_scale == 0 {
                last_scale
            } else {
                next_scale
            };
            last_scale = *item;
            Ok::<(), H264CodecError>(())
        })?;
        Ok(Self { scale, delta_scale })
    }
}
