use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use utils::traits::buffer::GenericSequencer;

use crate::errors::StreamCenterError;
use crate::gop::MediaFrame;

#[derive(Debug)]
pub struct MixQueue {
    media_frames: BTreeMap<(u64, u64), MediaFrame>, // (dts, seq)
    video_cnt: usize,
    audio_cnt: usize,
    pure_av_max_frame_count: usize,
    capacity: usize,
    seq_counter: u64,
    last_enqueue: Instant,
    drain_timeout: Option<Duration>,
    av_sync_tolerance_ns: u64, // Max allowed A/V skew before dropping/delaying
}

impl MixQueue {
    pub fn new(capacity: usize, pure_av_max_frame_count: usize) -> Self {
        assert!(capacity > 0);
        assert!(pure_av_max_frame_count > 0);
        Self {
            media_frames: BTreeMap::new(),
            pure_av_max_frame_count,
            capacity,
            video_cnt: 0,
            audio_cnt: 0,
            seq_counter: 0,
            last_enqueue: Instant::now(),
            drain_timeout: None,
            av_sync_tolerance_ns: 50_000_000, // 50ms by default
        }
    }

    pub fn with_drain_timeout(mut self, timeout: Duration) -> Self {
        self.drain_timeout = Some(timeout);
        self
    }

    pub fn with_av_sync_tolerance(mut self, tolerance: Duration) -> Self {
        self.av_sync_tolerance_ns = tolerance.as_nanos() as u64;
        self
    }

    fn current_av_skew(&self) -> Option<i64> {
        let first_audio = self.media_frames.iter().find(|(_, f)| f.is_audio());
        let first_video = self.media_frames.iter().find(|(_, f)| f.is_video());

        match (first_audio, first_video) {
            (Some((a_key, _)), Some((v_key, _))) => {
                let skew = a_key.0 as i64 - v_key.0 as i64;
                Some(skew)
            }
            _ => None,
        }
    }

    fn try_dump_one(&mut self) -> Option<MediaFrame> {
        if self.audio_cnt == 0 && self.video_cnt == 0 {
            return None;
        }

        let pure_av = self.video_cnt == 0 || self.audio_cnt == 0;
        let too_few_frames = self.video_cnt + self.audio_cnt < self.pure_av_max_frame_count;

        if pure_av && too_few_frames {
            if let Some(timeout) = self.drain_timeout {
                if self.last_enqueue.elapsed() < timeout {
                    return None;
                }
                // timeout expired -> flush anyway
            } else {
                return None;
            }
        }

        // --- A/V SYNC CONTROL ---
        if let Some(skew) = self.current_av_skew()
            && skew.abs() > self.av_sync_tolerance_ns as i64
        {
            // One track is way ahead -> drop the earlier frames to catch up
            if skew > 0 {
                // Audio ahead of video -> drop audio until we catch up
                if let Some(key) = self
                    .media_frames
                    .iter()
                    .find(|(_, f)| f.is_video())
                    .map(|(k, _)| *k)
                {
                    let dropped = self.media_frames.remove(&key).unwrap();
                    self.video_cnt = self.video_cnt.saturating_sub(1);
                    tracing::warn!("Dropping early video frame for A/V sync (skew={}ns)", skew);
                    return Some(dropped);
                }
            } else {
                // Video ahead of audio -> drop video until we catch up
                if let Some(key) = self
                    .media_frames
                    .iter()
                    .find(|(_, f)| f.is_audio())
                    .map(|(k, _)| *k)
                {
                    let dropped = self.media_frames.remove(&key).unwrap();
                    self.audio_cnt = self.audio_cnt.saturating_sub(1);
                    tracing::warn!("Dropping early audio frame for A/V sync (skew={}ns)", skew);
                    return Some(dropped);
                }
            }
        }

        // --- Normal frame popping ---
        if let Some((_, frame)) = self.media_frames.pop_first() {
            if frame.is_video() {
                self.video_cnt -= 1;
            } else {
                self.audio_cnt -= 1;
            }
            return Some(frame);
        }

        None
    }
}

impl GenericSequencer for MixQueue {
    type Error = StreamCenterError;
    type In = MediaFrame;
    type Out = MediaFrame;

    fn enqueue(&mut self, packet: Self::In) -> Result<(), Self::Error> {
        assert!(
            !packet.is_sequence_header(),
            "MixQueue does not support sequence header packets"
        );

        if self.media_frames.len() >= self.capacity {
            return Err(StreamCenterError::MixQueueFull(
                packet.get_codec_name().to_string(),
                self.capacity,
            ));
        }

        let key = (packet.get_decode_timestamp_ns(), self.seq_counter);
        self.seq_counter = self.seq_counter.wrapping_add(1);

        if let Some(old) = self.media_frames.insert(key, packet) {
            if old.is_video() {
                self.video_cnt = self.video_cnt.saturating_sub(1);
            } else if old.is_audio() {
                self.audio_cnt = self.audio_cnt.saturating_sub(1);
            }
            debug_assert!(false);
        }

        let inserted = self.media_frames.get(&key).unwrap();
        if inserted.is_video() {
            self.video_cnt += 1;
        } else if inserted.is_audio() {
            self.audio_cnt += 1;
        } else {
            unreachable!("MixQueue only supports audio and video packets");
        }

        self.last_enqueue = Instant::now();
        Ok(())
    }

    fn try_dump(&mut self) -> Vec<Self::Out> {
        let mut result = Vec::new();
        while let Some(frame) = self.try_dump_one() {
            result.push(frame);
        }
        result
    }
}
