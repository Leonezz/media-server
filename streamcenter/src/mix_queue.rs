use std::{cmp::max, collections::VecDeque};

use utils::traits::buffer::GenericSequencer;

use crate::{errors::StreamCenterError, gop::MediaFrame};

#[derive(Debug)]
pub struct MixQueue {
    pub video: VecDeque<MediaFrame>,
    pub audio: VecDeque<MediaFrame>,
    initial_buffering_frame_count: usize,
    max_video_frame_count: usize,
    max_audio_frame_count: usize,
}

impl MixQueue {
    pub fn new(
        max_video_frame_count: usize,
        max_audio_frame_count: usize,
        initial_buffering_frame_count: usize,
    ) -> Self {
        Self {
            video: VecDeque::new(),
            audio: VecDeque::new(),
            max_video_frame_count,
            max_audio_frame_count,
            initial_buffering_frame_count,
        }
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
        if packet.is_video() {
            if self.video.len() >= self.max_video_frame_count {
                return Err(StreamCenterError::MixQueueFull(
                    "video".to_owned(),
                    self.max_video_frame_count,
                ));
            }

            self.video.push_back(packet);
            Ok(())
        } else if packet.is_audio() {
            if self.audio.len() >= self.max_audio_frame_count {
                return Err(StreamCenterError::MixQueueFull(
                    "audio".to_owned(),
                    self.max_audio_frame_count,
                ));
            }

            self.audio.push_back(packet);
            return Ok(());
        } else {
            unreachable!("MixQueue only supports audio and video packets");
        }
    }
    fn try_dump(&mut self) -> Vec<Self::Out> {
        if self.video.len() + self.audio.len() < self.initial_buffering_frame_count {
            return vec![];
        }

        let (mut min_video_timestamp, mut min_audio_timestamp) = (u64::MAX, u64::MAX);
        for frame in &self.video {
            if frame.get_timestamp_ns() < min_video_timestamp {
                min_video_timestamp = frame.get_timestamp_ns();
            }
        }
        for frame in &self.audio {
            if frame.get_timestamp_ns() < min_audio_timestamp {
                min_audio_timestamp = frame.get_timestamp_ns();
            }
        }

        let min_timestamp = max(min_audio_timestamp, min_video_timestamp);
        let mut result = Vec::new();

        // Remove video frames with timestamp <= min_timestamp
        let mut i = 0;
        while i < self.video.len() {
            if self.video[i].get_timestamp_ns() <= min_timestamp {
                result.push(self.video.remove(i).unwrap());
            } else {
                i += 1;
            }
        }

        // Remove audio frames with timestamp <= min_timestamp
        let mut i = 0;
        while i < self.audio.len() {
            if self.audio[i].get_timestamp_ns() <= min_timestamp {
                result.push(self.audio.remove(i).unwrap());
            } else {
                i += 1;
            }
        }
        result.sort_by_key(|a| a.get_timestamp_ns());
        result
    }
}
