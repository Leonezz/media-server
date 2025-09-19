use num::ToPrimitive;
use utils::traits::dynamic_sized_packet::{DynamicSizedBitsPacket, DynamicSizedPacket};

use crate::codec::mpeg4_generic::parameters::RtpMpeg4Fmtp;

use super::{AuHeader, AuHeaderSection};

pub struct AuHeaderBitsCountWrapper<'a>(pub &'a AuHeader, pub &'a RtpMpeg4Fmtp);
impl<'a> DynamicSizedBitsPacket for AuHeaderBitsCountWrapper<'a> {
    fn get_packet_bits_count(&self) -> usize {
        let (value, param) = (self.0, self.1);
        let mut result = 0_u64;
        if let Some(size_length) = param.size_length
            && size_length > 0
            && value.au_size.is_some()
        {
            result += size_length;
        }
        if let Some(index_length) = param.index_length
            && index_length > 0
            && value.au_index.is_some()
        {
            result += index_length;
        }
        if let Some(index_delta_length) = param.index_delta_length
            && index_delta_length > 0
            && value.au_index_delta.is_some()
        {
            result += index_delta_length;
        }
        if let Some(cts_delta_length) = param.cts_delta_length
            && cts_delta_length > 0
        {
            result += if value.cts_delta.is_some() {
                cts_delta_length + 1
            } else {
                1
            };
        }
        if let Some(dts_delta_length) = param.dts_delta_length
            && dts_delta_length > 0
        {
            result += if value.dts_delta.is_some() {
                dts_delta_length + 1
            } else {
                1
            };
        }
        if let Some(rap) = param.random_access_indication
            && rap
        {
            result += 1;
        }
        if let Some(stream_state_idication) = param.stream_state_indication
            && stream_state_idication > 0
            && value.stream_state.is_some()
        {
            result += stream_state_idication;
        }

        result.to_usize().expect("integer overflow usize")
    }
}

pub struct AuHeaderSectionBytesCountWrapper<'a>(
    pub &'a AuHeaderSection,
    pub &'a RtpMpeg4Fmtp,
);

impl<'a> DynamicSizedPacket for AuHeaderSectionBytesCountWrapper<'a> {
    fn get_packet_bytes_count(&self) -> usize {
        let au_header_bits_cnt = self.0.au_headers.iter().fold(0, |prev, item| {
            AuHeaderBitsCountWrapper(item, self.1).get_packet_bits_count() + prev
        });
        au_header_bits_cnt.div_ceil(8) + 2
    }
}
