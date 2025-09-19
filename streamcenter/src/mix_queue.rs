use crate::{errors::StreamCenterError, gop::MediaFrame};
use std::collections::BTreeMap;
use utils::traits::buffer::GenericSequencer;

#[derive(Debug)]
pub struct MixQueue {
    pub media_frames: BTreeMap<u64, MediaFrame>,
    video_cnt: usize,
    audio_cnt: usize,
    pure_av_max_frame_count: usize,
    capacity: usize,
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
        }
    }

    fn try_dump_one(&mut self) -> Option<MediaFrame> {
        if self.audio_cnt == 0 && self.video_cnt == 0 {
            return None;
        }
        if (self.audio_cnt == 0 || self.video_cnt == 0)
            && self.video_cnt + self.audio_cnt < self.pure_av_max_frame_count
        {
            return None;
        }

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
        if self.media_frames.len() > self.capacity {
            return Err(StreamCenterError::MixQueueFull(
                packet.get_codec_name().to_string(),
                self.capacity,
            ));
        }

        if packet.is_video() {
            self.video_cnt += 1;
        } else if packet.is_audio() {
            self.audio_cnt += 1;
        } else {
            unreachable!("MixQueue only supports audio and video packets");
        }
        self.media_frames
            .insert(packet.get_decode_timestamp_ns(), packet);
        Ok(())
    }

    fn try_dump(&mut self) -> Vec<Self::Out> {
        let mut result = vec![];
        while let Some(frame) = self.try_dump_one() {
            result.push(frame);
        }
        result
    }
}
