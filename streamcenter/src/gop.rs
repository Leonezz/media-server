use std::collections::VecDeque;

use flv::tag::{
    audio_tag_header_info::AudioTagHeaderWithoutMultiTrack, on_meta_data::OnMetaData,
    video_tag_header_info::VideoTagHeaderWithoutMultiTrack,
};
use tokio_util::bytes::BytesMut;

use crate::{errors::StreamCenterResult, frame_info::MediaMessageRuntimeStat};

#[derive(Debug, Clone)]
pub enum MediaFrame {
    Video {
        runtime_stat: MediaMessageRuntimeStat,
        pts: u64,
        // NOTE - this tag_header is also included in the frame payload
        header: VideoTagHeaderWithoutMultiTrack,
        payload: BytesMut,
    },
    Audio {
        runtime_stat: MediaMessageRuntimeStat,
        pts: u64,
        // NOTE - this tag_header is also included in the frame payload
        header: AudioTagHeaderWithoutMultiTrack,
        payload: BytesMut,
    },
    Script {
        runtime_stat: MediaMessageRuntimeStat,
        pts: u64,
        // onMetaData should be the content of payload,
        // note the payload still holds all the bytes
        on_meta_data: Box<Option<OnMetaData>>,
        payload: BytesMut,
    },
}

impl MediaFrame {
    #[inline]
    pub fn is_video(&self) -> bool {
        matches!(self, MediaFrame::Video {
            runtime_stat: _,
            pts: _,
            header: _,
            payload: _,
        })
    }

    #[inline]
    pub fn is_audio(&self) -> bool {
        matches!(self, MediaFrame::Audio {
            runtime_stat: _,
            pts: _,
            header: _,
            payload: _,
        })
    }

    #[inline]
    pub fn is_script(&self) -> bool {
        matches!(self, MediaFrame::Script {
            runtime_stat: _,
            pts: _,
            payload: _,
            on_meta_data: _,
        })
    }

    #[inline]
    pub fn is_sequence_header(&self) -> bool {
        match self {
            MediaFrame::Audio {
                runtime_stat: _,
                pts: _,
                header,
                payload: _,
            } => header.is_sequence_header(),
            MediaFrame::Video {
                runtime_stat: _,
                pts: _,
                header,
                payload: _,
            } => header.is_sequence_header(),
            _ => false,
        }
    }

    #[inline]
    pub fn is_video_key_frame(&self) -> bool {
        match self {
            MediaFrame::Video {
                runtime_stat: _,
                pts: _,
                header,
                payload: _,
            } => header.is_key_frame(),
            _ => false,
        }
    }

    #[inline]
    pub fn get_frame_pts(&self) -> u64 {
        match self {
            MediaFrame::Audio {
                runtime_stat: _,
                pts,
                header: _,
                payload: _,
            }
            | MediaFrame::Video {
                runtime_stat: _,
                pts,
                header: _,
                payload: _,
            }
            | MediaFrame::Script {
                runtime_stat: _,
                pts,
                on_meta_data: _,
                payload: _,
            } => *pts,
        }
    }
}

#[derive(Debug)]
pub struct Gop {
    pub flv_frames: Vec<MediaFrame>,
    video_tag_cnt: usize,
    audio_tag_cnt: usize,
    meta_tag_cnt: usize,
    first_video_pts: u64,
    last_video_pts: u64,
}

impl Gop {
    pub fn new() -> Self {
        Self {
            flv_frames: Vec::new(),
            video_tag_cnt: 0,
            audio_tag_cnt: 0,
            meta_tag_cnt: 0,
            first_video_pts: 0,
            last_video_pts: 0,
        }
    }

    #[inline]
    pub fn get_video_frame_cnt(&self) -> usize {
        self.video_tag_cnt
    }

    #[inline]
    pub fn get_audio_frame_cnt(&self) -> usize {
        self.audio_tag_cnt
    }

    #[inline]
    pub fn get_meta_frame_cnt(&self) -> usize {
        self.meta_tag_cnt
    }

    #[inline]
    pub fn get_first_video_pts(&self) -> u64 {
        self.first_video_pts
    }

    #[inline]
    pub fn get_last_video_pts(&self) -> u64 {
        self.last_video_pts
    }

    pub fn append_flv_tag(&mut self, frame: MediaFrame) {
        match frame {
            MediaFrame::Video {
                pts,
                header: _,
                payload: _,
                runtime_stat: _,
            } => {
                self.video_tag_cnt += 1;
                if self.flv_frames.is_empty() {
                    self.first_video_pts = pts;
                }
                self.last_video_pts = pts;
            }
            MediaFrame::Audio {
                pts: _,
                header: _,
                payload: _,
                runtime_stat: _,
            } => self.audio_tag_cnt += 1,
            MediaFrame::Script {
                pts: _,
                payload: _,
                runtime_stat: _,
                on_meta_data: _,
            } => self.meta_tag_cnt += 1,
        }

        self.flv_frames.push(frame);
    }
}

impl Default for Gop {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct GopQueue {
    pub video_sequence_header: Option<MediaFrame>,
    pub audio_sequence_header: Option<MediaFrame>,
    pub script_frame: Option<MediaFrame>,
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
            script_frame: None,
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
    pub fn get_meta_frame_cnt(&self) -> usize {
        self.accumulate_gops(|gop| gop.get_meta_frame_cnt())
    }

    pub fn append_frame(&mut self, frame: MediaFrame) -> StreamCenterResult<()> {
        let mut is_sequence_header = false;
        let mut is_video = false;
        match frame {
            MediaFrame::Audio {
                pts: _,
                header,
                payload: _,
                runtime_stat: _,
            } => {
                if header.is_sequence_header() {
                    tracing::info!("audio header: {:?}", header);
                    self.audio_sequence_header = Some(frame.clone());
                    is_sequence_header = true
                }
            }
            MediaFrame::Video {
                pts: _,
                header,
                payload: _,
                runtime_stat: _,
            } => {
                is_video = true;
                if header.is_sequence_header() {
                    tracing::info!("video header: {:?}", header);
                    self.video_sequence_header = Some(frame.clone());
                    is_sequence_header = true;
                } else if header.is_key_frame() {
                    self.gops.push_back(Gop::new());
                }
            }
            MediaFrame::Script {
                runtime_stat: _,
                pts,
                ref on_meta_data,
                payload: _,
            } => {
                self.script_frame = Some(frame.clone());
                tracing::info!("meta, pts: {}, data: {:?}", pts, on_meta_data);
            }
        }

        if self.gops.is_empty() && is_video {
            self.dropped_video_cnt += 1;
            return Ok(());
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
                .append_flv_tag(frame);
            self.total_frame_cnt += 1;
        }

        Ok(())
    }
}
