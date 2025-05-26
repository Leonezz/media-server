use crate::codec::mpeg4_generic::{
    errors::{RtpMpeg4Error, RtpMpeg4Result},
    parameters::RtpMpeg4Fmtp,
};

use super::AuHeader;

#[derive(Debug, Default)]
pub struct AuHeaderBuilder {
    header: AuHeader,
}

impl AuHeaderBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn au_size(mut self, size: Option<u64>) -> Self {
        self.header.au_size = size;
        self
    }

    pub fn au_index(mut self, index: Option<u64>) -> Self {
        self.header.au_index = index;
        self
    }

    pub fn au_index_delta(mut self, index_delta: Option<u64>) -> Self {
        self.header.au_index_delta = index_delta;
        self
    }

    pub fn cts_delta(mut self, cts_delta: Option<u64>) -> Self {
        self.header.cts_delta = cts_delta;
        self
    }

    pub fn dts_delta(mut self, dts_delta: Option<u64>) -> Self {
        self.header.dts_delta = dts_delta;
        self
    }

    pub fn rap_flag(mut self, rap_flag: Option<bool>) -> Self {
        self.header.rap_flag = rap_flag;
        self
    }

    pub fn stream_state(mut self, stream_state: Option<u64>) -> Self {
        self.header.stream_state = stream_state;
        self
    }

    pub fn build(
        &self,
        params: &RtpMpeg4Fmtp,
        is_first_au: bool,
        is_fragmented: bool,
    ) -> RtpMpeg4Result<AuHeader> {
        if let Some(size_length) = params.size_length
            && size_length > 0
            && self.header.au_size.is_none()
        {
            return Err(RtpMpeg4Error::SyntaxError(format!(
                "no au size field found while sizeLength is provided: {}",
                params
            )));
        }

        let mut result = self.header.clone();
        if params.size_length.unwrap_or(0) == 0 {
            result.au_index = None;
        }

        if params.index_length.unwrap_or(0) == 0 {
            result.au_index = None;
        }

        if params.index_delta_length.unwrap_or(0) == 0 {
            result.au_index_delta = None;
        }

        if params.cts_delta_length.unwrap_or(0) == 0 {
            result.cts_delta = None;
        }

        if params.dts_delta_length.unwrap_or(0) == 0 {
            result.dts_delta = None;
        }

        if !params.random_access_indication.unwrap_or(false) {
            // no rap flag
            result.rap_flag = None;
        } else if result.rap_flag.is_none() {
            // must have rap flag
            result.rap_flag = Some(false);
        } else {
            // already have rap flag
            if !is_first_au && is_fragmented {
                // reset to false if non first fragment
                result.rap_flag = Some(false);
            }
        }

        if params.stream_state_indication.unwrap_or(0) == 0 {
            result.stream_state = None;
        }

        if is_first_au {
            result.au_index_delta = None;
            if params.index_length.unwrap_or(0) != 0 && result.au_index.is_none() {
                return Err(RtpMpeg4Error::SyntaxError(format!(
                    "no au index found while auIndexLength is provided: {}",
                    params
                )));
            }
        } else {
            result.au_index = None;
            if params.index_delta_length.unwrap_or(0) != 0 && result.au_index_delta.is_none() {
                return Err(RtpMpeg4Error::SyntaxError(format!(
                    "no au index delta found while auIndexDeltaLength is provided: {}",
                    params,
                )));
            }
        }

        if is_first_au || is_fragmented {
            result.cts_delta = None;
        }

        Ok(result)
    }
}
