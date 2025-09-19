pub mod reader;
pub mod writer;

use bitstream_io::BitRead;
use utils::traits::dynamic_sized_packet::DynamicSizedBitsPacket;

use crate::{
    errors::{H264CodecError, H264CodecResult},
    exp_golomb::{find_se_bits_count, read_se},
};

#[derive(Debug, Clone)]
pub struct SeqScalingMatrix {
    pub(crate) seq_scaling_list_present_flag: [bool; 12], // u(1)
    /// if seq_scaling_list_present_flag[i]
    pub scaling_list_4x4: [ScalingListRaw<16>; 6], // TODO-
    pub scaling_list_8x8: [ScalingListRaw<64>; 6],
}

#[derive(Debug, Clone, Copy)]
pub struct ScalingListRaw<const C: usize> {
    pub(crate) delta_scale: [Option<i64>; C], // for write only
    pub scale: [i64; C],
}

impl<const C: usize> DynamicSizedBitsPacket for ScalingListRaw<C> {
    fn get_packet_bits_count(&self) -> usize {
        self.delta_scale.iter().fold(0, |prev, item| {
            prev + item.map(|v| find_se_bits_count(v).unwrap()).unwrap_or(0)
        })
    }
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
        reader: &mut R,
        use_default_scaling_matrix_flag: &mut bool,
    ) -> H264CodecResult<Self> {
        let mut last_scale: i64 = 8;
        let mut next_scale: i64 = 8;
        let mut scale = [0; C];
        let mut delta_scale = [None; C];
        scale.iter_mut().enumerate().try_for_each(|(i, item)| {
            if next_scale != 0 {
                let delta = read_se(reader)?;
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
