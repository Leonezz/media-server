use std::fmt::Debug;

pub mod audio;
pub mod errors;
pub mod video;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MediaFrameTimestamp {
    pub(self) presentation_timestamp_nano: u64,
    pub(self) decode_timestamp_nano: u64,
}

impl MediaFrameTimestamp {
    pub fn new(pts_nano: u64, dts_nano: u64) -> Self {
        Self {
            presentation_timestamp_nano: pts_nano,
            decode_timestamp_nano: dts_nano,
        }
    }

    pub fn with_timestamp_nano(ts: u64) -> Self {
        Self::new(ts, ts)
    }

    pub fn with_timestamp_ms(ts: u64) -> Self {
        let ts = ts.checked_mul(1_000_000).unwrap();
        Self::with_timestamp_nano(ts)
    }

    pub fn set_pts(&mut self, pts_nano: u64) -> &mut Self {
        self.presentation_timestamp_nano = pts_nano;
        self
    }

    pub fn set_pts_ms(&mut self, pts_ms: u64) -> &mut Self {
        let ts = pts_ms.checked_mul(1_000_000).unwrap();
        self.set_pts(ts)
    }

    pub fn set_dts(&mut self, dts_nano: u64) -> &mut Self {
        self.decode_timestamp_nano = dts_nano;
        self
    }

    pub fn set_dts_ms(&mut self, dts_ms: u64) -> &mut Self {
        let ts = dts_ms.checked_mul(1_000_000).unwrap();
        self.set_dts(ts)
    }

    pub fn apply_offset_nano(&mut self, cts_nano: u64) -> &mut Self {
        self.presentation_timestamp_nano = self
            .presentation_timestamp_nano
            .checked_add(cts_nano)
            .unwrap();
        self
    }

    pub fn apply_offset_ms(&mut self, cts_ms: u64) -> &mut Self {
        let ts = cts_ms.checked_mul(1_000_000).unwrap();
        self.apply_offset_nano(ts)
    }

    pub fn pts(&self) -> u64 {
        self.presentation_timestamp_nano
    }

    pub fn pts_ms(&self) -> u64 {
        self.pts().checked_div(1_000_000).unwrap()
    }

    pub fn dts(&self) -> u64 {
        self.decode_timestamp_nano
    }

    pub fn dts_ms(&self) -> u64 {
        self.dts().checked_div(1_000_000).unwrap()
    }

    pub fn to_debug_str(&self) -> String {
        format!(
            "pts nano: {}, dts nano: {}",
            self.presentation_timestamp_nano, self.decode_timestamp_nano
        )
    }
}

impl Debug for MediaFrameTimestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_debug_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    SequenceStart,
    CodedFrames,
    KeyFrame,
    SequenceEnd,
}
