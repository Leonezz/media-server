use std::collections::VecDeque;

use crate::frame_info::FrameData;

#[derive(Debug)]
pub struct Gop {
    frames: Vec<FrameData>,
    video_frame_cnt: usize,
    audio_frame_cnt: usize,
    aggregate_frame_cnt: usize,
    meta_frame_cnt: usize,
    first_video_pts: u64,
    last_video_pts: u64,
}

impl Gop {
    pub fn new() -> Self {
        Self {
            frames: Vec::new(),
            video_frame_cnt: 0,
            audio_frame_cnt: 0,
            aggregate_frame_cnt: 0,
            meta_frame_cnt: 0,
            first_video_pts: 0,
            last_video_pts: 0,
        }
    }

    #[inline]
    pub fn get_video_frame_cnt(&self) -> usize {
        self.video_frame_cnt
    }

    #[inline]
    pub fn get_audio_frame_cnt(&self) -> usize {
        self.audio_frame_cnt
    }

    #[inline]
    pub fn get_aggregate_frame_cnt(&self) -> usize {
        self.aggregate_frame_cnt
    }

    #[inline]
    pub fn get_meta_frame_cnt(&self) -> usize {
        self.meta_frame_cnt
    }

    #[inline]
    pub fn get_first_video_pts(&self) -> u64 {
        self.first_video_pts
    }

    #[inline]
    pub fn get_last_video_pts(&self) -> u64 {
        self.last_video_pts
    }

    pub fn append_frame(&mut self, frame: FrameData) {
        match frame {
            FrameData::Video { meta, data: _ } => {
                self.video_frame_cnt += 1;
                if self.frames.is_empty() {
                    self.first_video_pts = meta.pts;
                }
                self.last_video_pts = meta.pts;
            }
            FrameData::Audio { meta: _, data: _ } => self.audio_frame_cnt += 1,
            FrameData::Aggregate { meta: _, data: _ } => self.aggregate_frame_cnt += 1,
            FrameData::Meta {
                timestamp: _,
                data: _,
            } => self.meta_frame_cnt += 1,
        }

        self.frames.push(frame);
    }
}

#[derive(Debug)]
pub struct GopCache {
    pub video_sequence_header: Option<FrameData>,
    pub audio_sequence_header: Option<FrameData>,
    gops: VecDeque<Gop>,
    total_frame_cnt: usize,
    max_duration_ms: u64,
    max_frame_cnt: usize,
}

impl GopCache {
    pub fn new(max_duration_ms: u64, max_frame_cnt: usize) -> Self {
        Self {
            video_sequence_header: None,
            audio_sequence_header: None,
            gops: VecDeque::new(),
            max_duration_ms,
            max_frame_cnt,
            total_frame_cnt: 0,
        }
    }

    #[inline]
    pub fn get_gops_cnt(&self) -> usize {
        self.gops.len()
    }

    #[inline]
    fn accumulate_gops<'a, F>(&'a self, f: F) -> usize
    where
        F: Fn(&'a Gop) -> usize,
    {
        let mut result = 0;
        for gop in &self.gops {
            result += f(gop);
        }
        result
    }

    #[inline]
    pub fn get_video_frame_cnt(&self) -> usize {
        self.accumulate_gops(|gop| gop.get_video_frame_cnt())
    }

    #[inline]
    pub fn get_audio_frame_cut(&self) -> usize {
        self.accumulate_gops(|gop| gop.get_audio_frame_cnt())
    }

    #[inline]
    pub fn get_aggregate_frame_cnt(&self) -> usize {
        self.accumulate_gops(|gop| gop.get_aggregate_frame_cnt())
    }

    #[inline]
    pub fn get_meta_frame_cnt(&self) -> usize {
        self.accumulate_gops(|gop| gop.get_meta_frame_cnt())
    }

    pub fn append_frame(&mut self, frame: FrameData) {
        if self.gops.is_empty() {
            let sh = frame.clone();
            match sh {
                FrameData::Video { meta, data } => {
                    self.video_sequence_header = Some(FrameData::Video { meta, data })
                }
                FrameData::Audio { meta, data } => {
                    self.audio_sequence_header = Some(FrameData::Audio { meta, data })
                }
                _ => {}
            }
        }
        if self.gops.is_empty() || frame.is_video_idr() {
            self.gops.push_back(Gop::new());
        }

        let first_pts = self
            .gops
            .front()
            .expect("this cannot be empty")
            .get_first_video_pts();
        let last_pts = self
            .gops
            .back()
            .expect("this cannot be empty")
            .get_last_video_pts();

        if ((last_pts > first_pts && (last_pts - first_pts) >= self.max_duration_ms)
            || self.total_frame_cnt >= self.max_frame_cnt)
            && self.gops.len() > 1
        {
            self.gops.pop_front();
        }

        self.gops
            .back_mut()
            .expect("this cannot be empty")
            .append_frame(frame);
        self.total_frame_cnt += 1;
        tracing::info!(
            "gop cache got frame, gop count: {}, frame_cnt: {}",
            self.gops.len(),
            self.total_frame_cnt,
        )
    }
}