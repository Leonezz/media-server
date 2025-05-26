use std::{cmp, collections::VecDeque};

use codec_common::{
    FrameType,
    audio::{AudioCodecCommon, AudioFrameInfo, SoundInfoCommon},
    video::VideoFrameInfo,
};
use codec_h264::nalu_type::NALUType;
use num::ToPrimitive;
use stream_center::gop::MediaFrame;
use tokio_util::bytes::{BufMut, BytesMut};
use utils::traits::{
    buffer::GenericSequencer, dynamic_sized_packet::DynamicSizedPacket, writer::WriteTo,
};

use crate::{
    codec::{
        h264::packet::sequencer::RtpH264BufferItem,
        mpeg4_generic::packet::sequencer::RtpMpeg4GenericBufferItem,
    },
    errors::RtpError,
    sequence_number::SequenceNumber,
};

use super::RtpTrivialPacket;

#[derive(Debug)]
pub enum RtpBufferVideoItem {
    H264(RtpH264BufferItem),
}

#[derive(Debug)]
pub enum RtpBufferAudioItem {
    AAC(RtpMpeg4GenericBufferItem),
}

#[derive(Debug)]
pub enum RtpBufferItem {
    Video(RtpBufferVideoItem),
    Audio(RtpBufferAudioItem),
}

pub struct RtpBufferItemToMeidiaFrameComposer {
    sps: Option<RtpBufferItem>,
    pps: Option<RtpBufferItem>,
    timestamp_base: Option<u32>,
    clock_rate: u32,
}

impl RtpBufferItemToMeidiaFrameComposer {
    pub fn new(clock_rate: u32, sps: Option<RtpBufferItem>, pps: Option<RtpBufferItem>) -> Self {
        Self {
            sps,
            pps,
            timestamp_base: None,
            clock_rate,
        }
    }
}

impl RtpBufferItem {
    pub fn get_timestamp(&self) -> u32 {
        match self {
            Self::Audio(audio) => match audio {
                RtpBufferAudioItem::AAC(aac) => aac.access_unit.timestamp,
            },
            Self::Video(video) => match video {
                RtpBufferVideoItem::H264(h264) => h264
                    .rtp_header
                    .timestamp
                    .checked_add(h264.timestamp_offset.unwrap_or(0))
                    .unwrap(),
            },
        }
    }
    pub fn get_sequence_number(&self) -> u16 {
        match self {
            Self::Audio(audio) => match audio {
                RtpBufferAudioItem::AAC(aac) => aac.rtp_header.sequence_number,
            },
            Self::Video(video) => match video {
                RtpBufferVideoItem::H264(h264) => h264.rtp_header.sequence_number,
            },
        }
    }

    pub fn is_video(&self) -> bool {
        matches!(self, Self::Video(_))
    }

    pub fn is_audio(&self) -> bool {
        matches!(self, Self::Audio(_))
    }

    pub fn get_packet_type(&self) -> String {
        match self {
            Self::Audio(_) => "audio".to_owned(),
            Self::Video(_) => "video".to_owned(),
        }
    }

    pub fn get_video_nal_type(&self) -> Option<NALUType> {
        match self {
            Self::Video(video) => match video {
                RtpBufferVideoItem::H264(h264) => Some(h264.nal_unit.header.nal_unit_type),
            },
            _ => None,
        }
    }

    pub fn to_media_frame(&self, timestamp_base: u32, clock_rate: u32) -> MediaFrame {
        let timestamp_nano = self
            .get_timestamp()
            .checked_sub(timestamp_base)
            .and_then(|v| v.to_u64())
            .and_then(|v| v.checked_mul(1_000_000_000)) // to nano seconds
            .and_then(|v| v.checked_div(clock_rate.to_u64().unwrap()))
            .unwrap();
        match self {
            RtpBufferItem::Audio(audio) => match audio {
                RtpBufferAudioItem::AAC(aac) => {
                    let mut bytes = BytesMut::zeroed(aac.access_unit.get_packet_bytes_count());
                    aac.access_unit
                        .write_to(&mut bytes.as_mut().writer())
                        .unwrap();
                    MediaFrame::Audio {
                        frame_info: AudioFrameInfo {
                            codec_id: AudioCodecCommon::AAC,
                            frame_type: FrameType::CodedFrames,
                            sound_info: SoundInfoCommon {
                                sound_rate: codec_common::audio::SoundRateCommon::KHZ44,
                                sound_size: codec_common::audio::SoundSizeCommon::Bit8,
                                sound_type: codec_common::audio::SoundTypeCommon::Stereo,
                            },
                            timestamp_nano,
                        },
                        payload: bytes.freeze(),
                    }
                }
            },
            RtpBufferItem::Video(video) => match video {
                RtpBufferVideoItem::H264(h264) => {
                    let is_idr = h264.nal_unit.header.nal_unit_type == NALUType::IDRSlice;
                    MediaFrame::Video {
                        frame_info: VideoFrameInfo {
                            codec_id: codec_common::video::VideoCodecCommon::AVC,
                            frame_type: if is_idr {
                                FrameType::KeyFrame
                            } else {
                                FrameType::CodedFrames
                            },
                            timestamp_nano,
                        },
                        payload: codec_common::video::VideoFrameUnit::H264 {
                            nal_units: vec![h264.nal_unit.clone()],
                        },
                    }
                }
            },
        }
    }
}

pub trait RtpBufferedSequencer {
    fn enqueue(&mut self, packet: RtpTrivialPacket) -> Result<(), RtpError>;
    fn try_dump(&mut self) -> Vec<RtpBufferItem>;
}

/// RtpTrivialSequencer takes rtp packets from outside systems,
/// and one can obtain RtpTrivialPacket from it with sequence number being continuous
pub struct RtpTrivialSequencer {
    capacity: usize,
    initial_buffer_packets: usize,
    initial_buffering: bool,
    next_sequence_number: SequenceNumber,
    buffer: VecDeque<RtpTrivialPacket>,
}

impl RtpTrivialSequencer {
    pub fn new(capacity: usize, initial_buffer_packets: usize) -> Self {
        Self {
            capacity,
            initial_buffer_packets,
            initial_buffering: true,
            next_sequence_number: SequenceNumber::new(0, 0),
            buffer: VecDeque::with_capacity(capacity),
        }
    }

    pub fn timestamp_minmax(&self) -> Option<(u32, u32)> {
        if self.buffer.is_empty() {
            return None;
        }

        Some(
            self.buffer
                .iter()
                .fold((u32::MAX, u32::MIN), |(min, max), item| {
                    (
                        cmp::min(min, item.header.timestamp),
                        cmp::max(max, item.header.timestamp),
                    )
                }),
        )
    }

    pub fn smallest_sequence_number_item_index(&self) -> Option<(u16, usize)> {
        if self.buffer.is_empty() {
            return None;
        }
        let mut result = (u16::MAX, 0);
        for (i, item) in self.buffer.iter().enumerate() {
            if item.header.sequence_number < result.0 {
                result = (item.header.sequence_number, i);
            }
        }
        Some(result)
    }
}

impl GenericSequencer for RtpTrivialSequencer {
    type In = RtpTrivialPacket;
    type Out = RtpTrivialPacket;
    type Error = RtpError;
    fn enqueue(&mut self, packet: Self::In) -> Result<(), Self::Error> {
        self.buffer.push_back(packet);
        Ok(())
    }

    fn try_dump(&mut self) -> Vec<Self::Out> {
        let _span =
            tracing::debug_span!("rtp sequencer dump", queue_size = self.buffer.len()).entered();
        if self.buffer.is_empty() {
            return vec![];
        }
        if self.initial_buffering && self.buffer.len() < self.initial_buffer_packets {
            return vec![];
        }

        let mut result = vec![];
        if self.initial_buffering {
            self.initial_buffering = false;
            let (min_seq, index) = self.smallest_sequence_number_item_index().unwrap();
            self.next_sequence_number.set_round(0);
            self.next_sequence_number.set_number(min_seq);
            self.next_sequence_number.add_number(1);
            result.push(self.buffer.remove(index).unwrap());
        }
        while let Some((min_seq, index)) = self.smallest_sequence_number_item_index() {
            if self.next_sequence_number.number() < min_seq && self.buffer.len() < self.capacity / 4
            {
                tracing::debug!(
                    "interleaved rtp packets detected, waiting. expected seq: {}, min seq: {}",
                    self.next_sequence_number.number(),
                    min_seq
                );
                break;
            }
            if self.next_sequence_number.number() > min_seq {
                let item = self.buffer.remove(index).unwrap();
                if self.next_sequence_number.number() - min_seq < 10000 {
                    tracing::warn!("outdated rtp packets detected: {:?}", item.header);
                } else {
                    tracing::trace!(
                        "rtp sequence number might wrapped, adjuest next_sequence_number: {}",
                        min_seq
                    );
                    result.push(item);
                    self.next_sequence_number.set_number(min_seq);
                    self.next_sequence_number.add_number(1);
                }
                continue;
            }
            // here: min_seq = self.next_seqence_number
            result.push(self.buffer.remove(index).unwrap());
            self.next_sequence_number.set_number(min_seq);
            self.next_sequence_number.add_number(1);
        }

        while self.buffer.len() > self.capacity
            && let Some((min_seq, index)) = self.smallest_sequence_number_item_index()
        {
            tracing::warn!(
                "sequencer buffer overflow, dump smallest sequence number item: {}",
                min_seq
            );
            result.push(self.buffer.remove(index).unwrap());
            self.next_sequence_number.set_number(min_seq);
            self.next_sequence_number.add_number(1);
        }
        result
    }
}
