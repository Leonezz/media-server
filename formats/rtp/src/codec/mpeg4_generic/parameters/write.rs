use std::fmt;

use super::RtpMpeg4OutOfBandParams;

impl From<&RtpMpeg4OutOfBandParams> for String {
    fn from(value: &RtpMpeg4OutOfBandParams) -> Self {
        let mut params = Vec::new();

        params.push(format!("profile-level-id={}", value.profile_level_id));
        params.push(format!("config={}", String::from_utf8_lossy(&value.config)));
        params.push(format!("mode={}", <&str>::from(&value.mode)));

        if let Some(object_type) = value.object_type {
            params.push(format!("objectType={}", object_type));
        }
        if let Some(constant_size) = value.constant_size {
            params.push(format!("constantSize={}", constant_size));
        }
        if let Some(constant_duration) = value.constant_duration {
            params.push(format!("constantDuration={}", constant_duration));
        }
        if let Some(max_displacement) = value.max_displacement {
            params.push(format!("maxDisplacement={}", max_displacement));
        }
        if let Some(de_interleave_buffer_size) = value.de_interleave_buffer_size {
            params.push(format!(
                "de-interleaveBufferSize={}",
                de_interleave_buffer_size
            ));
        }
        if let Some(size_length) = value.size_length {
            params.push(format!("sizeLength={}", size_length));
        }
        if let Some(index_length) = value.index_length {
            params.push(format!("indexLength={}", index_length));
        }
        if let Some(index_delta_length) = value.index_delta_length {
            params.push(format!("indexDeltaLength={}", index_delta_length));
        }
        if let Some(cts_delta_length) = value.cts_delta_length {
            params.push(format!("CTSDeltaLength={}", cts_delta_length));
        }
        if let Some(dts_delta_length) = value.dts_delta_length {
            params.push(format!("DTSDeltaLength={}", dts_delta_length));
        }
        if let Some(random_access_indication) = value.random_access_indication {
            params.push(format!(
                "randomAccessIndication={}",
                random_access_indication
            ));
        }
        if let Some(stream_state_indication) = value.stream_state_indication {
            params.push(format!("streamStateIndication={}", stream_state_indication));
        }
        if let Some(auxiliary_data_size_length) = value.auxiliary_data_size_length {
            params.push(format!(
                "auxiliaryDataSizeLength={}",
                auxiliary_data_size_length
            ));
        }

        params.join(";")
    }
}

impl fmt::Display for RtpMpeg4OutOfBandParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str: String = self.into();
        write!(f, "{str}")
    }
}
