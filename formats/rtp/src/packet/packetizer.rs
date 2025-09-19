use codec_common::{
    audio::AudioCodecCommon,
    video::{H264VideoConfig, VideoConfig, VideoFrameUnit},
};
use codec_h264::nalu::NalUnit;
use stream_center::gop::MediaFrame;
use tokio_util::bytes::{Bytes, BytesMut};

use crate::{errors::RtpError, header::RtpHeader};

use super::RtpTrivialPacket;

#[derive(Debug, Default)]
pub struct RtpTrivialPacketBuilder {
    header: RtpHeader,
    payload: BytesMut,
}

impl RtpTrivialPacketBuilder {
    pub fn header(mut self, header: RtpHeader) -> Self {
        self.header = header;
        self
    }
    pub fn version(mut self, version: u8) -> Self {
        self.header.version = version;
        self
    }

    pub fn payload(mut self, payload: &[u8]) -> Self {
        self.payload.extend_from_slice(payload);
        self
    }

    pub fn build(self) -> RtpTrivialPacket {
        RtpTrivialPacket::new(self.header, self.payload.freeze())
    }
}

#[derive(Debug)]
pub struct RtpTrivialPacketizerH264Item {
    pub nalus: Vec<NalUnit>,
}

#[derive(Debug)]
pub struct RtpTrivialPacketizerAACItem {
    pub access_units: Vec<Bytes>,
}

#[derive(Debug)]
pub enum RtpPacketizerVideoItem {
    H264(RtpTrivialPacketizerH264Item),
}

#[derive(Debug)]
pub enum RtpPacketizerAudioItem {
    AAC(RtpTrivialPacketizerAACItem),
}

#[derive(Debug)]
pub enum RtpPacketizerItem {
    Video(RtpPacketizerVideoItem),
    Audio(RtpPacketizerAudioItem),
}

impl RtpPacketizerItem {
    pub fn from_media_frame(frame: MediaFrame) -> Option<Self> {
        match frame {
            MediaFrame::Video {
                frame_info: _,
                payload,
            } => match payload {
                VideoFrameUnit::H264 { nal_units } => Some(RtpPacketizerItem::Video(
                    RtpPacketizerVideoItem::H264(RtpTrivialPacketizerH264Item { nalus: nal_units }),
                )),
                _ => unimplemented!("unsupported video format {:?}", payload),
            },
            MediaFrame::Audio {
                frame_info,
                payload,
            } => match frame_info.codec_id {
                AudioCodecCommon::AAC => Some(RtpPacketizerItem::Audio(
                    RtpPacketizerAudioItem::AAC(RtpTrivialPacketizerAACItem {
                        access_units: vec![payload],
                    }),
                )),
                _ => unimplemented!("unsupported audio format {:?}", frame_info),
            },
            MediaFrame::VideoConfig {
                timestamp_nano: _,
                config,
            } => match *config {
                VideoConfig::H264(H264VideoConfig {
                    sps,
                    pps,
                    sps_ext: _,
                    avc_decoder_configuration_record: _,
                }) => {
                    let mut nal_units = Vec::new();
                    if let Some(sps) = sps {
                        nal_units.push((&sps).into());
                    }
                    if let Some(pps) = pps {
                        nal_units.push((&pps).into());
                    }
                    Some(RtpPacketizerItem::Video(RtpPacketizerVideoItem::H264(
                        RtpTrivialPacketizerH264Item { nalus: nal_units },
                    )))
                }
            },
            MediaFrame::AudioConfig {
                timestamp_nano: _,
                sound_info: _,
                config: _,
            } => {
                tracing::debug!("audio config frame, ignore");
                None
            }
            MediaFrame::Script {
                timestamp_nano: _,
                on_meta_data: _,
                payload: _,
            } => {
                tracing::debug!("script frame, ignore");
                None
            }
        }
    }
}

pub trait RtpTrivialPacketPacketizer {
    fn set_rtp_header(&mut self, header: RtpHeader);
    fn set_frame_timestamp(&mut self, timestamp: u64);
    fn get_rtp_clockrate(&self) -> u64;
    fn rtp_header(&self) -> &RtpHeader;
    fn packetize(&mut self, item: RtpPacketizerItem) -> Result<(), RtpError>;
    fn build(&mut self) -> Result<Vec<RtpTrivialPacket>, RtpError>;
}

pub fn wallclock_to_rtp_timestamp(
    ts_ms: u64,
    base_wallclock_ms: u64,
    base_rtp_ts: u64,
    clockrate: u64,
) -> u64 {
    let delta_ms = ts_ms.saturating_sub(base_wallclock_ms);
    let delta_rtp = (delta_ms * clockrate) / 1000;
    base_rtp_ts.wrapping_add(delta_rtp)
}
