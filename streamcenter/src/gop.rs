use std::{collections::VecDeque, io::Cursor};

use tokio_util::bytes::BytesMut;

use crate::{errors::StreamCenterResult, frame_info::FrameData};

#[derive(Debug)]
pub struct Gop {
    pub frames: Vec<FrameData>,
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
pub struct GopQueue {
    pub video_sequence_header: Option<FrameData>,
    pub audio_sequence_header: Option<FrameData>,
    pub gops: VecDeque<Gop>,
    total_frame_cnt: u64,
    max_duration_ms: u64,
    max_frame_cnt: u64,
    dropped_gops_cnt: u64,
    dropped_video_cnt: u64,
    dropped_audio_cnt: u64,
}

impl GopQueue {
    pub fn new(max_duration_ms: u64, max_frame_cnt: u64) -> Self {
        Self {
            video_sequence_header: None,
            audio_sequence_header: None,
            gops: VecDeque::new(),
            max_duration_ms,
            max_frame_cnt,
            total_frame_cnt: 0,
            dropped_gops_cnt: 0,
            dropped_video_cnt: 0,
            dropped_audio_cnt: 0,
        }
    }

    #[inline]
    pub fn get_gops_cnt(&self) -> usize {
        self.gops.len()
    }

    #[inline]
    pub fn get_dropped_gop_cnt(&self) -> u64 {
        self.dropped_gops_cnt
    }

    #[inline]
    pub fn get_dropped_video_cnt(&self) -> u64 {
        self.dropped_video_cnt
    }

    #[inline]
    pub fn get_dropped_audio_cnt(&self) -> u64 {
        self.dropped_audio_cnt
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

    pub fn append_frame(&mut self, frame: FrameData) -> StreamCenterResult<()> {
        let mut is_sequence_header = false;
        match frame {
            FrameData::Audio { meta, data: _ } => {
                if meta.tag_header.is_sequence_header() {
                    self.audio_sequence_header = Some(frame.clone());
                    is_sequence_header = true
                }
            }
            FrameData::Video { meta, data: _ } => {
                if meta.tag_header.is_sequence_header() {
                    self.video_sequence_header = Some(frame.clone());
                    is_sequence_header = true;
                } else if meta.tag_header.is_key_frame() {
                    self.gops.push_back(Gop::new());
                }
            }
            _ => {}
        }

        if self.gops.is_empty() {
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
            let dropped = self.gops.pop_front();
            if let Some(gop) = dropped {
                self.dropped_gops_cnt += 1;
                self.dropped_video_cnt += gop.get_video_frame_cnt() as u64;
                self.dropped_audio_cnt += gop.get_audio_frame_cnt() as u64;
            }
        }

        if !is_sequence_header {
            self.gops
                .back_mut()
                .expect("this cannot be empty")
                .append_frame(frame);
            self.total_frame_cnt += 1;
        }

        Ok(())
    }
}
