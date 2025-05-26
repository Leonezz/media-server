use std::{
    io,
    net::{IpAddr, SocketAddr},
    pin::Pin,
    time::Duration,
};

use codec_common::audio::SoundInfoCommon;
use codec_h264::avc_decoder_configuration_record::AvcDecoderConfigurationRecord;
use futures::SinkExt;
use num::ToPrimitive;
use rtp_formats::{
    codec::{
        h264::{packet::sequencer::RtpH264Sequencer, paramters::RtpH264Fmtp},
        mpeg4_generic::{packet::sequencer::RtpMpeg4GenericSequencer, parameters::RtpMpeg4Fmtp},
    },
    packet::{
        RtpTrivialPacket,
        sequencer::{RtpBufferedSequencer, RtpTrivialSequencer},
    },
    rtcp::RtcpPacket,
};
use rtp_session::{
    session::{RtpSession, RtpSessionCommand},
    simple_statistics::RtpSessionSimpleStatistics,
};
use rtsp_formats::{
    header::transport::TransportHeader, interleaved::RtspInterleavedPacket,
    sdp_extension::attribute::RtspSDPControl,
};
use sdp_formats::{
    attributes::{SDPAttribute, fmtp::FormatParameters, rtpmap::RtpMap},
    session::{SDPBandwidthType, SDPMediaDescription, SDPMediaType},
};

use stream_center::gop::MediaFrame;
use tokio::sync::mpsc::{Sender, error::TryRecvError};
use tracing::{Instrument, Span};
use unified_io::{UnifiedIO, channel::ChannelIo, udp::UdpIO};
use url::Url;
use utils::traits::buffer::GenericSequencer;
use uuid::Uuid;

use crate::{
    SERVER_AGENT,
    errors::{RtspServerError, RtspServerResult},
};

#[derive(Debug)]
struct StreamProperties {
    pub(crate) stream_name: String,
    pub(crate) sub_stream_name: String,
    pub(crate) uri: Url,
}

#[derive(Debug)]
pub enum RtspSessionCommand {
    Stop,
    Start,
    Rtp(RtpTrivialPacket),
    Rtcp(RtcpPacket),
}

pub struct RtspMediaSession {
    peer_addr: SocketAddr,
    session_id: Uuid,
    stream_properities: StreamProperties,
    bandwidth: Option<u64>,
    control: RtspSDPControl,
    rtpmap: RtpMap,
    fmtp: Option<FormatParameters>,
    media_description: SDPMediaDescription,
    pub(crate) transport: TransportHeader,

    pub(crate) local_rtp_port: u16,
    pub(crate) local_rtcp_port: u16,

    rtp_session_command_tx: tokio::sync::mpsc::UnboundedSender<RtpSessionCommand>,
    rtp_rx: tokio::sync::mpsc::UnboundedReceiver<RtpTrivialPacket>,

    rtp_thread_handle: Option<tokio::task::JoinHandle<()>>,

    interleaved_rtp_io: Option<(u8, ChannelIo)>,
    interleaved_rtcp_io: Option<(u8, ChannelIo)>,

    rtsp_session_command_rx: tokio::sync::mpsc::UnboundedReceiver<RtspSessionCommand>,

    rtp_unpacker: Option<Box<dyn RtpBufferedSequencer + Send>>, // use Option only for debug and dev
    rtp_trivial_sequencer: Option<RtpTrivialSequencer>,

    media_frame_sender: Sender<MediaFrame>,
    first_rtp_packet_timestamp: Option<u32>,
    rtp_clockrate: u32,
}

impl RtspMediaSession {
    pub async fn new(
        peer_addr: SocketAddr,
        uri: Url,
        session_id: Uuid,
        media_description: SDPMediaDescription,
        transport: TransportHeader,
        rtsp_command_rx: tokio::sync::mpsc::UnboundedReceiver<RtspSessionCommand>,
        media_frame_sender: Sender<MediaFrame>,
    ) -> RtspServerResult<Self> {
        let control = Self::extract_control_attribute(&media_description)?;
        let rtpmap: RtpMap = (&media_description).try_into()?;
        let rtp_clockrate = rtpmap.clock_rate.to_u32().unwrap();
        let fmtp = Self::extrace_fmtp(&media_description).ok();
        let bandwidth = Self::extrace_bandwidth(&media_description).ok();

        if transport.profile.is_none() {
            return Err(RtspServerError::InvalidTransport(format!(
                "transport profile is none, {:?}",
                &transport
            )));
        }

        let (rtp_command_tx, rtp_command_rx) =
            tokio::sync::mpsc::unbounded_channel::<RtpSessionCommand>();
        let (rtp_tx, rtp_rx) = tokio::sync::mpsc::unbounded_channel::<RtpTrivialPacket>();

        let ((rtp_io, rtp_port), (rtcp_io, rtcp_port)) =
            Self::create_rtp_io_pair(peer_addr, &transport).await?;
        let rtp_session = RtpSession::new(
            Some(SERVER_AGENT.to_owned()),
            bandwidth.unwrap_or(500),
            rtpmap.clock_rate,
            rtp_command_rx,
            rtp_tx,
        );

        tracing::info!("new rtsp media session is created");

        let stream_name = uri.path();
        let rtp_session_span = tracing::debug_span!("rtp session",
            rtsp_session_id = %session_id,
            rtsp_uri = %uri,
            rtsp_control = %control,
        );
        let rtp_handle =
            Self::start_rtp_session(rtp_session, rtp_io, rtcp_io, rtp_session_span).await?;
        let unpacker = Self::create_rtp_unpacker(
            media_description.media_line.media_type.clone(),
            &rtpmap,
            &fmtp,
        )
        .ok();

        Ok(Self {
            peer_addr,
            stream_properities: StreamProperties {
                stream_name: stream_name.to_owned(),
                sub_stream_name: control.to_string(),
                uri,
            },
            session_id,
            bandwidth,
            control,
            rtpmap,
            fmtp,
            media_description,
            transport,
            rtp_session_command_tx: rtp_command_tx,
            rtp_rx,
            rtp_thread_handle: Some(rtp_handle),

            local_rtp_port: rtp_port,
            local_rtcp_port: rtcp_port,

            interleaved_rtcp_io: None,
            interleaved_rtp_io: None,

            rtsp_session_command_rx: rtsp_command_rx,

            rtp_unpacker: unpacker,
            rtp_trivial_sequencer: Some(RtpTrivialSequencer::new(200, 10)),
            media_frame_sender,

            first_rtp_packet_timestamp: None,
            rtp_clockrate,
        })
    }

    async fn start_rtp_session(
        rtp_session: RtpSession,
        rtp_io: Pin<Box<dyn UnifiedIO>>,
        rtcp_io: Pin<Box<dyn UnifiedIO>>,
        span: Span,
    ) -> RtspServerResult<tokio::task::JoinHandle<()>> {
        span.in_scope(|| {
            tracing::info!("rtp session is about to run");
        });
        let res = tokio::task::spawn(
            async move {
                match rtp_session
                    .with_observer(Box::new(RtpSessionSimpleStatistics::new()))
                    .await
                    .run(rtp_io, rtcp_io)
                    .await
                {
                    Ok(()) => {
                        tracing::info!("rtp session successfully closed");
                    }
                    Err(err) => {
                        tracing::error!("rtp session error: {:?}", err);
                    }
                };
            }
            .instrument(span),
        );
        Ok(res)
    }

    fn create_rtp_unpacker(
        media_type: SDPMediaType,
        rtpmap: &RtpMap,
        fmtp: &Option<FormatParameters>,
    ) -> RtspServerResult<Box<dyn RtpBufferedSequencer + Send>> {
        match rtpmap.encoding_name.to_lowercase().as_str() {
            "h264" => {
                tracing::info!(
                    "got H264 encoding, creating h264 sequencer, rtpmap: {}, fmtp: {:?}",
                    rtpmap,
                    fmtp
                );
                if fmtp.is_none() {
                    return Err(RtspServerError::InvalidParamForRtpUnpacker(
                        "unable to create h264 rtp unpacker with fmtp being None".to_owned(),
                    ));
                }
                let h264_fmtp: RtpH264Fmtp = fmtp.clone().unwrap().params.parse()?;
                tracing::info!("fmtp params for h264 parsed from sdp: {:?}", h264_fmtp);
                if h264_fmtp.packetization_mode.is_none() {
                    return Err(RtspServerError::InvalidParamForRtpUnpacker(
                        "unable to create h264 rtp unpacker with packetization mode being None"
                            .to_owned(),
                    ));
                }
                let unpacker =
                    RtpH264Sequencer::new(h264_fmtp.packetization_mode.unwrap(), h264_fmtp.into());
                Ok(Box::new(unpacker))
            }
            "mpeg4-generic" => {
                if matches!(media_type, SDPMediaType::Audio) {
                    let params = if let Some(fmtp) = fmtp {
                        let params = fmtp.params.parse()?;
                        tracing::info!(
                            "fmtp params for mpeg4-generic parsed from sdp: {:?}",
                            params
                        );
                        params
                    } else {
                        let mut params = RtpMpeg4Fmtp::default();
                        params
                            .set_mode(rtp_formats::codec::mpeg4_generic::parameters::Mode::AAChbr);
                        params.reset_default();
                        tracing::warn!(
                            "no fmtp params found in sdp, set default for mpeg4-generic: {:?}",
                            params
                        );
                        params
                    };
                    let unpacker = RtpMpeg4GenericSequencer::new(params, 10000, 10);
                    Ok(Box::new(unpacker))
                } else {
                    Err(RtspServerError::InvalidParamForRtpUnpacker(format!(
                        "get mpeg4-generic format but not for audio: {}",
                        media_type
                    )))
                }
            }
            _ => {
                tracing::warn!("unknown encoding_name: {}", rtpmap.encoding_name);
                Err(RtspServerError::InvalidEncodingName(
                    rtpmap.encoding_name.clone(),
                ))
            }
        }
    }

    async fn create_rtp_io_pair(
        peer_addr: SocketAddr,
        transport: &TransportHeader,
    ) -> RtspServerResult<(
        (Pin<Box<dyn UnifiedIO>>, u16),
        (Pin<Box<dyn UnifiedIO>>, u16),
    )> {
        let (rtp_io, rtp_port, rtcp_io, rtcp_port) = if transport.profile.as_ref().unwrap().is_udp()
        {
            let (client_rtp_port, client_rtcp_port) =
                transport
                    .client_port
                    .ok_or(RtspServerError::InvalidTransport(format!(
                        "transport client port is none, {:?}",
                        transport
                    )))?;

            let ((rtp_io, rtp_port), (rtcp_io, rtcp_port)) =
                Self::create_udp_io_pair(peer_addr.ip(), client_rtp_port, client_rtcp_port).await?;
            tracing::info!(
                "created udp io, rtp port: {}, rtcp port: {}",
                rtp_port,
                rtcp_port
            );

            (rtp_io, rtp_port, rtcp_io, rtcp_port)
        } else if transport.profile.as_ref().unwrap().is_tcp() {
            todo!()
        } else {
            return Err(RtspServerError::InvalidTransport(format!(
                "unsupported transport profile: {:?}",
                transport
            )));
        };
        Ok(((Box::pin(rtp_io), rtp_port), (Box::pin(rtcp_io), rtcp_port)))
    }

    async fn create_udp_io_pair(
        peer_ip: IpAddr,
        peer_rtp_port: u16,
        peer_rtcp_port: u16,
    ) -> RtspServerResult<((UdpIO, u16), (UdpIO, u16))> {
        let (rtp_port, rtp_io) =
            UdpIO::new_with_remote_addr(1000, SocketAddr::new(peer_ip, peer_rtp_port))
                .await
                .map_err(|err| {
                    tracing::error!("failed to create udp io: {}", err);
                    RtspServerError::IoError(io::Error::other(format!(
                        "failed to create udp io: {}",
                        err
                    )))
                })?;
        let (rtcp_port, rtcp_io) =
            UdpIO::new_with_remote_addr(rtp_port + 1, SocketAddr::new(peer_ip, peer_rtcp_port))
                .await
                .map_err(|err| {
                    tracing::error!("failed to create udp io: {}", err);
                    RtspServerError::IoError(io::Error::other(format!(
                        "failed to create udp io: {}",
                        err
                    )))
                })?;
        Ok(((rtp_io, rtp_port), (rtcp_io, rtcp_port)))
    }

    pub async fn run(&mut self) -> RtspServerResult<()> {
        let span = tracing::debug_span!("rtsp media session",
            session_id = %self.session_id,
            uri = %self.stream_properities.uri,
            control = %self.control,
            transport = %self.transport,
        );
        span.in_scope(|| {
            match &mut self
                .rtp_session_command_tx
                .send(RtpSessionCommand::Start {})
            {
                Ok(()) => {
                    tracing::info!("rtp session is started");
                    Ok(())
                }
                Err(err) => {
                    tracing::error!("failed to start rtp session: {:?}", err);
                    Err(RtspServerError::IoError(io::Error::other(format!(
                        "failed to start rtp session: {:?}",
                        err
                    ))))
                }
            }
        })?;
        loop {
            self.process_commands(&span)?;
            match tokio::time::timeout(Duration::from_secs(2), self.read_packet(&span)).await {
                Err(_) => {}
                Ok(res) => res?,
            }
        }
    }

    async fn read_packet(&mut self, span: &Span) -> RtspServerResult<()> {
        match self.rtp_rx.recv().await {
            None => Err(RtspServerError::IoError(io::Error::other(
                "rtp data channel from rtp session to rtsp media session is closed unexpected",
            ))),
            Some(data) => span.in_scope(async || {
                let packets = if let Some(trivial_sequencer) = &mut self.rtp_trivial_sequencer {
                    trivial_sequencer.enqueue(data).unwrap();
                    trivial_sequencer.try_dump()
                } else {
                    vec![data]
                };

                if let Some(unpacker) = &mut self.rtp_unpacker {
                    for packet in packets {
                        match unpacker.enqueue(packet) {
                            Err(err) => {
                                tracing::error!(
                                    "push new rtp packet to rtp sequencer failed with error: {}",
                                    err
                                );
                            }
                            Ok(()) => {
                                tracing::trace!("push new rtp packet to rtp sequencer succeed");
                            }
                        }
                    }
                    let ready_packets = unpacker.try_dump();
                    if self.first_rtp_packet_timestamp.is_none() && !ready_packets.is_empty() {
                        self.first_rtp_packet_timestamp = Some(ready_packets[0].get_timestamp());
                        // time to send audio/video configs
                        if let Some(fmtp) = self.fmtp.as_ref() {
                            match self.rtpmap.encoding_name.to_lowercase().as_str() {
                                "h264" => {
                                    let h264_fmtp: RtpH264Fmtp = fmtp.params.parse()?;
                                    let config: AvcDecoderConfigurationRecord =
                                        (&h264_fmtp).try_into()?;
                                    tracing::debug!("make avc decoder configuration record from fmtp: {:#?}", config);
                                    let h264_sequence_header = MediaFrame::VideoConfig {
                                        timestamp_nano: 0,
                                        config: Box::new(config.into()),
                                    };
                                    
                                    self.media_frame_sender.send(h264_sequence_header).await.map_err(|err| {
                                        tracing::error!("send h264 sequence header to stream center failed: {}", err);
                                        RtspServerError::IoError(io::Error::other(format!("channel send h264 sequence header to stream center failed: {}", err)))
                                    })?;
                                    tracing::info!("publish h264 video sequence header to stream center succeed");
                                }
                                "mpeg4-generic" => {
                                    let aac_fmtp: RtpMpeg4Fmtp = fmtp.params.parse()?;
                                    if let Some(audio_specific_config) = aac_fmtp.aac_audio_specific_config {
                                        let aac_sequence_header = MediaFrame::AudioConfig { timestamp_nano: 0, sound_info: SoundInfoCommon { sound_rate: codec_common::audio::SoundRateCommon::KHZ44, sound_size: codec_common::audio::SoundSizeCommon::Bit8, sound_type: codec_common::audio::SoundTypeCommon::Stereo }, config:  Box::new(audio_specific_config.into())};
                                        self.media_frame_sender.send(aac_sequence_header).await.map_err(|err| {
                                            tracing::error!("send aac sequence header to stream center failed: {}", err);
                                            RtspServerError::IoError(io::Error::other(format!("channel send aac sequence header to stream center failed: {}", err)))
                                        })?;
                                        tracing::info!("publish aac audio sequence header to stream center succeed");
                                    } else {
                                        tracing::warn!("no aac audio specific config found");
                                    }
                                }
                                _ => {
                                    unimplemented!()
                                }
                            }
                        }
                    }
                    for packet in ready_packets {
                        // match &packet {
                        //     RtpBufferItem::Video(video_item) => {
                        //         if let RtpBufferVideoItem::H264(h264) = video_item {

                        //         }
                        //     }
                        //     _ => {}
                        // }
                        match self.media_frame_sender.send(packet.to_media_frame(self.first_rtp_packet_timestamp.unwrap(), self.rtp_clockrate)).await {
                            Ok(()) => {}
                            Err(err) => {
                                tracing::error!(
                                    "send unpacked rtp media packets to rtsp session failed: {}",
                                    err
                                );
                                return Err(RtspServerError::IoError(io::Error::other(format!(
                                    "send unpacked rtp media packets to rtsp session failed: {}",
                                    err
                                ))));
                            }
                        }
                    }
                } else {
                    tracing::error!(
                        "no rtp_unpacker found, drop all {} rtp packets",
                        packets.len()
                    );
                    return Err(RtspServerError::NoRtpUnpacker(format!(
                        "no rtp unpacker for: {:?}",
                        self.rtpmap
                    )));
                }
                Ok(())
            }).await,
        }
    }

    fn process_commands(&mut self, span: &Span) -> RtspServerResult<()> {
        let command = self.rtsp_session_command_rx.try_recv();
        span.in_scope(|| match command {
            Err(TryRecvError::Disconnected) => {
                tracing::warn!("rtsp session command channel closed");
                Err(RtspServerError::IoError(io::Error::other(
                    "rtsp session command channel closed",
                )))
            }
            Err(_) => Ok(()),
            Ok(command) => match command {
                RtspSessionCommand::Start => Ok(()),
                RtspSessionCommand::Stop => {
                    tracing::info!("rtsp session is stopping");
                    self.rtp_session_command_tx
                        .send(RtpSessionCommand::Stop)
                        .map_err(|err| {
                            tracing::error!("failed to stop rtp session: {:?}", err);
                            RtspServerError::IoError(io::Error::other(format!(
                                "failed to stop rtp session: {:?}",
                                err
                            )))
                        })?;
                    Err(RtspServerError::GracefulExit)
                }
                RtspSessionCommand::Rtp(packet) => self
                    .rtp_session_command_tx
                    .send(RtpSessionCommand::Rtp(packet))
                    .map_err(|err| {
                        tracing::error!("failed to send rtp packet: {:?}", err);
                        RtspServerError::IoError(io::Error::other(format!(
                            "failed to send rtp packet: {:?}",
                            err
                        )))
                    }),
                RtspSessionCommand::Rtcp(packet) => self
                    .rtp_session_command_tx
                    .send(RtpSessionCommand::Rtcp(packet))
                    .map_err(|err| {
                        tracing::error!("failed to send rtcp packet: {:?}", err);
                        RtspServerError::IoError(io::Error::other(format!(
                            "failed to send rtcp packet: {:?}",
                            err
                        )))
                    }),
            },
        })
    }

    fn extract_control_attribute(
        media_description: &SDPMediaDescription,
    ) -> RtspServerResult<RtspSDPControl> {
        let control = media_description.attributes.iter().find_map(|attr| {
            if let SDPAttribute::Trivial(attr) = attr
                && attr.name == "control"
            {
                Some(RtspSDPControl::try_from(attr))
            } else {
                None
            }
        });
        if control.is_none() {
            tracing::warn!("media control attribute not found");
            return Err(RtspServerError::InvalidMediaDescription(
                "media control attribute not found".to_owned(),
            ));
        }

        Ok(control.unwrap()?)
    }

    fn extrace_fmtp(media_description: &SDPMediaDescription) -> RtspServerResult<FormatParameters> {
        let fmtp = media_description.attributes.iter().find_map(|attr| {
            if let SDPAttribute::Fmtp(fmtp) = attr {
                Some(fmtp)
            } else {
                None
            }
        });
        if fmtp.is_none() {
            tracing::warn!("media fmtp attribute not found");
            return Err(RtspServerError::InvalidMediaDescription(
                "media fmtp attribute not found".to_owned(),
            ));
        }
        Ok(fmtp.unwrap().clone())
    }

    fn extrace_bandwidth(media_description: &SDPMediaDescription) -> RtspServerResult<u64> {
        let bandwidth = if media_description.bandwidth.is_empty() {
            tracing::warn!("no bandwidth attribute found");
            None
        } else {
            let bandwidth = &media_description.bandwidth[0];
            if !matches!(bandwidth.bw_type, SDPBandwidthType::AS) {
                tracing::warn!("unsupported bandwidth type: {}", bandwidth.bw_type);
                None
            } else {
                Some(bandwidth.bandwidth)
            }
        };

        if bandwidth.is_none() {
            tracing::warn!("no bandwidth attribute found");
            return Err(RtspServerError::InvalidMediaDescription(
                "no bandwidth attribute found".to_owned(),
            ));
        }
        Ok(bandwidth.unwrap())
    }

    pub fn session_id(&self) -> Uuid {
        self.session_id
    }

    pub fn media_type(&self) -> &SDPMediaType {
        &self.media_description.media_line.media_type
    }

    pub async fn handle_interleaved_packet(&mut self, packet: RtspInterleavedPacket) {
        if self.interleaved_rtp_io.is_none() && self.interleaved_rtcp_io.is_none() {
            return;
        }
        if let Some((id, io)) = &mut self.interleaved_rtp_io
            && packet.channel_id == *id
        {
            if let Err(err) = io.send(packet.payload).await {
                tracing::error!("failed to write interleaved packet: {}", err);
            }
        } else if let Some((id, io)) = &mut self.interleaved_rtcp_io
            && packet.channel_id == *id
        {
            if let Err(err) = io.send(packet.payload).await {
                tracing::error!("failed to write interleaved packet: {}", err);
            }
        } else {
            tracing::warn!(
                "unknown interleaved packet channel id: {}",
                packet.channel_id
            );
        }
    }
}
