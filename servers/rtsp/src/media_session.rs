use std::{
    io, net::{IpAddr, SocketAddr}, pin::Pin, sync::atomic::{AtomicBool, Ordering}, time::Duration
};
use codec_aac::mpeg4_configuration::audio_specific_config::AudioSpecificConfig;
use codec_h264::avc_decoder_configuration_record::AvcDecoderConfigurationRecord;
use futures::SinkExt;
use rtp_formats::{
    codec::{
        h264::{packet::{packetizer::RtpH264PacketPacketizer, sequencer::RtpH264Sequencer}, paramters::RtpH264Fmtp},
        mpeg4_generic::{packet::{packetizer::RtpMpeg4GenericPacketPacketizer, sequencer::RtpMpeg4GenericSequencer}, parameters::RtpMpeg4Fmtp},
    }, packet::{packetizer::{RtpPacketizerItem, RtpTrivialPacketPacketizer}, sequencer::{RtpBufferedSequencer, RtpTrivialSequencer}, RtpTrivialPacket}, payload_types::rtp_payload_type::get_rtp_clockrate, rtcp::RtcpPacket
};
use rtp_session::{
    session::{RtpSession, RtpSessionCommand},
    simple_statistics::RtpSessionSimpleStatistics,
};
use rtsp_formats::{
    header::transport::{TransportHeader, TransportProtocol}, interleaved::RtspInterleavedPacket,
    sdp_extension::attribute::RtspSDPControl,
};
use sdp_formats::{
    attributes::{fmtp::FormatParameters, rtpmap::RtpMap, SDPAttribute}, session::{SDPBandwidthType, SDPMediaDescription, SDPMediaType}
};
use stream_center::{gop::MediaFrame};
use tokio::sync::broadcast::error::TryRecvError;
use tracing::{Instrument, Span};
use unified_io::{UnifiedIO, channel::ChannelIo, udp::UdpIO};
use url::Url;
use utils::{random::random_u32, traits::buffer::GenericSequencer};
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

#[derive(Debug, Clone)]
pub enum RtspSessionCommand {
    Stop,
    Start,
    Rtp(RtpTrivialPacket),
    Rtcp(RtcpPacket),
}

enum RuntimeHandler {
    Play{
        media_frame_receiver: tokio::sync::mpsc::Receiver<MediaFrame>,
        rtp_packetizer: Box<dyn RtpTrivialPacketPacketizer + Send>,
    },
    Publish{
        media_frame_sender: tokio::sync::mpsc::Sender<MediaFrame>,
        rtp_receiver: tokio::sync::mpsc::Receiver<RtpTrivialPacket>,
        rtp_sequencer: RtpTrivialSequencer,
        rtp_unpacker: Box<dyn RtpBufferedSequencer + Send>,
        
        control: Box<RtspSDPControl>,
        bandwidth: Option<u64>,
        rtpmap: RtpMap,
        fmtp: Option<FormatParameters>,
        media_description: Box<SDPMediaDescription>,
    },
    None,
}

pub struct RtspMediaSession {
    peer_addr: SocketAddr,
    session_id: String,
    stream_properities: StreamProperties,

    pub(crate) local_rtp_port: u16,
    pub(crate) local_rtcp_port: u16,

    pub(crate) transport: TransportHeader,

    interleaved_rtp_io: Option<(u8, ChannelIo)>,
    interleaved_rtcp_io: Option<(u8, ChannelIo)>,

    rtp_session_command_tx: tokio::sync::mpsc::Sender<RtpSessionCommand>,
    rtsp_session_command_rx: tokio::sync::broadcast::Receiver<RtspSessionCommand>,

    media_type: SDPMediaType,
    session_handler: RuntimeHandler,

    first_rtp_packet_timestamp: Option<u32>,
    rtp_clockrate: u64,
    ssrc: u32,
}

impl RtspMediaSession {
    #[allow(clippy::too_many_arguments)]
    pub async fn new_play_session(
        peer_addr: SocketAddr,
        uri: Url,
        control: &RtspSDPControl,
        media_sdp: &SDPMediaDescription,
        rtpmap: &RtpMap,
        session_id: String,
        transport: TransportHeader,
        rtsp_command_rx: tokio::sync::broadcast::Receiver<RtspSessionCommand>,
        media_frame_receiver: tokio::sync::mpsc::Receiver<MediaFrame>,
    ) -> RtspServerResult<Self> {
        if transport.profile.is_none() || transport.client_port.is_none() {
            return Err(RtspServerError::InvalidTransport(format!(
                "transport profile or client port is none, {:?}",
                &transport
            )));
        }
        let fmtp = media_sdp.get_fmtp();
        if fmtp.is_none() {
            tracing::error!("fmtp not found in media attributes");
            return Err(RtspServerError::InvalidMediaDescription("fmtp not found in media description".to_string()));
        }
        let ssrc = random_u32();
        let rtp_packetizer = Self::create_rtp_packetizer(ssrc, &fmtp.unwrap(), rtpmap.encoding_name.clone())?;
        let (rtp_command_tx, rtp_command_rx) =
            tokio::sync::mpsc::channel::<RtpSessionCommand>(1000);
        
        let (client_rtp_port, client_rtcp_port) = transport.client_port.unwrap();
        let ((rtp_io, rtp_port), (rtcp_io, rtcp_port)) =
            Self::create_rtp_io_pair(
                peer_addr,
                client_rtp_port,
                client_rtcp_port,
                transport.profile.unwrap()
            ).await?;
        tracing::debug!("new rtsp play session with rtp port: {}, rtcp port: {}, client rtp port: {}, client rtcp port: {}",
            rtp_port, rtcp_port, client_rtp_port, client_rtcp_port);
        let rtp_clockrate = get_rtp_clockrate(&rtpmap.encoding_name).unwrap();
        let rtp_session = RtpSession::new(
            ssrc,
            Some(SERVER_AGENT.to_owned()),
            10000,
            rtp_clockrate,
            rtp_command_rx,
            None,
        );
        tracing::info!("new rtsp media play session is created");

        let stream_name = uri.path();
        let rtp_session_span = tracing::debug_span!("rtp play session",
            rtsp_session_id = %session_id,
            rtsp_uri = %uri,
            rtsp_control = %control,
        );
        Self::start_rtp_session(true, rtp_session, rtp_io, rtcp_io, rtp_session_span).await?;
        Ok(Self {
            peer_addr,
            stream_properities: StreamProperties {
                stream_name: stream_name.to_owned(),
                sub_stream_name: "".to_owned(),
                uri,
            },
            session_id,
            transport,
            rtp_session_command_tx: rtp_command_tx,

            local_rtp_port: rtp_port,
            local_rtcp_port: rtcp_port,

            interleaved_rtcp_io: None,
            interleaved_rtp_io: None,

            rtsp_session_command_rx: rtsp_command_rx,
            media_type: media_sdp.media_line.media_type.clone(),
            session_handler: RuntimeHandler::Play {
                media_frame_receiver,
                rtp_packetizer
            },

            first_rtp_packet_timestamp: None,
            rtp_clockrate,
            ssrc,
        })

    }
    pub async fn new_publish_session(
        peer_addr: SocketAddr,
        uri: Url,
        session_id: String,
        media_description: SDPMediaDescription,
        transport: TransportHeader,
        rtsp_command_rx: tokio::sync::broadcast::Receiver<RtspSessionCommand>,
        media_frame_sender: tokio::sync::mpsc::Sender<MediaFrame>,
    ) -> RtspServerResult<Self> {
        let control = Self::extract_control_attribute(&media_description)?;
        let rtpmap: RtpMap = media_description.get_rtp_map().ok_or(RtspServerError::InvalidMediaDescription(
            format!("no rtpmap found in media description: {}", media_description)
        ))?;
        let fmtp = media_description.get_fmtp();
        let bandwidth = Self::extract_bandwidth(&media_description).ok();

        if transport.profile.is_none() || transport.client_port.is_none() {
            return Err(RtspServerError::InvalidTransport(format!(
                "transport profile or client port is none, {:?}",
                &transport
            )));
        }

        let unpacker = Self::create_rtp_unpacker(
            media_description.media_line.media_type.clone(),
            &rtpmap,
            &fmtp,
        )?;

        let (rtp_command_tx, rtp_command_rx) =
            tokio::sync::mpsc::channel::<RtpSessionCommand>(1000);
        let (rtp_tx, rtp_rx) = tokio::sync::mpsc::channel::<RtpTrivialPacket>(1000);

        let (client_rtp_port, client_rtcp_port) = transport.client_port.unwrap();
        let ((rtp_io, rtp_port), (rtcp_io, rtcp_port)) =
            Self::create_rtp_io_pair(
                peer_addr,
                client_rtp_port,
                client_rtcp_port,
                transport.profile.unwrap()
            ).await?;
        tracing::debug!("new rtsp publish session with rtp port: {}, rtcp port: {}, client rtp port: {}, client rtcp port: {}",
            rtp_port, rtcp_port, client_rtp_port, client_rtcp_port);
        let ssrc = random_u32();
        let rtp_session = RtpSession::new(
            ssrc,
            Some(SERVER_AGENT.to_owned()),
            bandwidth.unwrap_or(500),
            rtpmap.clock_rate,
            rtp_command_rx,
            Some(rtp_tx),
        );

        tracing::info!("new rtsp media publish session is created");

        let stream_name = uri.path();
        let rtp_session_span = tracing::debug_span!("rtp publish session",
            rtsp_session_id = %session_id,
            rtsp_uri = %uri,
            rtsp_control = %control,
        );
        Self::start_rtp_session(false, rtp_session, rtp_io, rtcp_io, rtp_session_span).await?;

        Ok(Self {
            peer_addr,
            stream_properities: StreamProperties {
                stream_name: stream_name.to_owned(),
                sub_stream_name: control.to_string(),
                uri,
            },
            session_id,
            transport,
            rtp_session_command_tx: rtp_command_tx,

            local_rtp_port: rtp_port,
            local_rtcp_port: rtcp_port,

            interleaved_rtcp_io: None,
            interleaved_rtp_io: None,

            rtsp_session_command_rx: rtsp_command_rx,
            media_type: media_description.media_line.media_type.clone(),
            rtp_clockrate: rtpmap.clock_rate,
            session_handler: RuntimeHandler::Publish {
                media_frame_sender,
                rtp_receiver: rtp_rx,
                rtp_sequencer: RtpTrivialSequencer::new(200, 10),
                rtp_unpacker: unpacker,
                control: Box::new(control),
                bandwidth,
                rtpmap,
                fmtp,
                media_description: Box::new(media_description)
            },

            first_rtp_packet_timestamp: None,
            ssrc,
        })
    }

    async fn start_rtp_session(
        send: bool,
        rtp_session: RtpSession,
        rtp_io: Pin<Box<dyn UnifiedIO>>,
        rtcp_io: Pin<Box<dyn UnifiedIO>>,
        span: Span,
    ) -> RtspServerResult<tokio::task::JoinHandle<()>> {
        span.in_scope(|| {
            tracing::info!("rtp session is about to run, is sending session: {}", send);
        });
        let res = tokio::task::spawn(
            async move {
                match rtp_session
                    .with_observer(Box::new(RtpSessionSimpleStatistics::new()))
                    .await
                    .run(send, rtp_io, rtcp_io)
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

    fn create_rtp_packetizer(
        ssrc: u32,
        fmtp: &FormatParameters,
        encoding_name: String,
    ) -> RtspServerResult<Box<dyn RtpTrivialPacketPacketizer + Send>> {
        tracing::info!("got {} encoding, creating packetizer with fmtp: {}", encoding_name, fmtp);
        match encoding_name.to_lowercase().as_str() {
            "h264" => {
                let h264_fmtp: RtpH264Fmtp = fmtp.params.parse()?;
                let packetizer = RtpH264PacketPacketizer::new(
                    1400, h264_fmtp.packetization_mode.unwrap_or_default(), ssrc
                );
                Ok(Box::new(packetizer))
            },
            "mpeg4-generic" => {
                let aac_fmtp: RtpMpeg4Fmtp = fmtp.params.parse()?;
                let packetizer = RtpMpeg4GenericPacketPacketizer::new(
                    1400, aac_fmtp, ssrc
                );
                Ok(Box::new(packetizer))
            }
            _ => {
                tracing::warn!("unknown encoding_name: {}", encoding_name);
                Err(RtspServerError::InvalidEncodingName(encoding_name))
            }
        }
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
                    RtpH264Sequencer::new(
                        h264_fmtp.packetization_mode.unwrap(), 
                        (&h264_fmtp).into(), 
                        h264_fmtp.sprop_parameter_sets.as_ref().and_then(|v| v.sps.clone()), 
                        h264_fmtp.sprop_parameter_sets.as_ref().and_then(|v| v.pps.clone()),
                    );
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
                        let params = RtpMpeg4Fmtp::default();
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
        peer_rtp_port: u16,
        peer_rtcp_port: u16,
        protocol: TransportProtocol,
    ) -> RtspServerResult<(
        (Pin<Box<dyn UnifiedIO>>, u16),
        (Pin<Box<dyn UnifiedIO>>, u16),
    )> {
        let (rtp_io, rtp_port, rtcp_io, rtcp_port) = if protocol.is_udp()
        {
            let ((rtp_io, rtp_port), (rtcp_io, rtcp_port)) =
                Self::create_udp_io_pair(peer_addr.ip(), peer_rtp_port, peer_rtcp_port).await?;
            tracing::info!(
                "created udp io, rtp port: {}, rtcp port: {}",
                rtp_port,
                rtcp_port
            );

            (rtp_io, rtp_port, rtcp_io, rtcp_port)
        } else if protocol.is_tcp() {
            todo!()
        } else {
            return Err(RtspServerError::InvalidTransport(format!(
                "unsupported protocol: {:?}",
                protocol
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
            transport = %self.transport,
        );
        span.in_scope(async || {
            match &mut self
                .rtp_session_command_tx
                .send(RtpSessionCommand::Start {}).await
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
        }).await?;
        loop {
            self.process_commands(&span).await?;
            match &mut self.session_handler {
                RuntimeHandler::Play { media_frame_receiver, rtp_packetizer } => {
                    match tokio::time::timeout(
                        Duration::from_secs(2),
                        Self::process_play(
                            &span,
                            media_frame_receiver,
                            rtp_packetizer,
                            &mut self.rtp_session_command_tx
                        )).await
                    {
                        Err(_) => {},
                        Ok(res) => res?,
                    }
                }
                RuntimeHandler::Publish {
                    media_frame_sender,
                    rtp_receiver,
                    rtp_sequencer,
                    rtp_unpacker,
                    control: _,
                    bandwidth: _,
                    rtpmap,
                    fmtp,
                    media_description: _
                } => {
                    match tokio::time::timeout(
                        Duration::from_secs(2),
                        Self::process_publish(
                            &span,
                            rtp_receiver,
                            rtp_sequencer,
                            rtp_unpacker,
                            media_frame_sender,
                            &mut self.first_rtp_packet_timestamp,
                            fmtp,
                            rtpmap,
                            self.rtp_clockrate)
                    ).await {
                        Err(_) => {}
                        Ok(res) => res?,
                    }
                }
                RuntimeHandler::None => {
                    tracing::warn!("no session handler, rtsp media session is idle");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
            
        }
    }

    async fn process_play(
        span: &Span,
        media_frame_receiver: &mut tokio::sync::mpsc::Receiver<MediaFrame>,
        rtp_packetizer: &mut Box<dyn RtpTrivialPacketPacketizer + Send>,
        rtp_sender: &mut tokio::sync::mpsc::Sender<RtpSessionCommand>,
    ) -> RtspServerResult<()> {
        match media_frame_receiver.recv().await {
            None => Err(RtspServerError::IoError(io::Error::other(
                "media frame channel from stream center to rtsp media session is closed unexpected",
            ))),
            Some(frame) => span.in_scope(async || {
                rtp_packetizer.set_frame_timestamp(frame.get_timestamp_ns().checked_div(1_000_000).unwrap());
                if let Some(item) = RtpPacketizerItem::from_media_frame(frame) {
                rtp_packetizer.packetize(item).inspect_err(|err| {
                    tracing::error!("error while packetizing media frame to rtp: {}", err);
                })?;
                let packets = rtp_packetizer.build().inspect_err(|err| {
                    tracing::error!("error while building rtp packets from packetizer: {}", err);
                })?;
                for packet in packets {
                    match rtp_sender.send(RtpSessionCommand::Rtp(packet)).await {
                        Ok(()) => {}
                        Err(err) => {
                            tracing::error!(
                                "send rtp packet to rtp session failed: {}",
                                err
                            );
                            return Err(RtspServerError::IoError(io::Error::other(format!(
                                "send rtp packet to rtp session failed: {}",
                                err
                            ))));
                        }
                    }
                }
                }
                Ok(())
            }).await,
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn process_publish(
        span: &Span,
        rtp_rx: &mut tokio::sync::mpsc::Receiver<RtpTrivialPacket>,
        rtp_sequencer: &mut RtpTrivialSequencer,
        rtp_unpacker: &mut Box<dyn RtpBufferedSequencer + Send>,
        media_frame_sender: &mut tokio::sync::mpsc::Sender<MediaFrame>,
        first_rtp_timestamp: &mut Option<u32>,
        fmtp: &Option<FormatParameters>,
        rtpmap: &RtpMap,
        rtp_clockrate: u64,
    ) -> RtspServerResult<()> {
        match rtp_rx.recv().await {
            None => Err(RtspServerError::IoError(io::Error::other(
                "rtp data channel from rtp session to rtsp media session is closed unexpected",
            ))),
            Some(data) => span.in_scope(async || {
                rtp_sequencer.enqueue(data).unwrap();
                let packets =  rtp_sequencer.try_dump();

                for packet in packets {
                    if let Err(err) = rtp_unpacker.enqueue(packet) {
                        tracing::error!(
                            "push new rtp packet to rtp sequencer failed with error: {}",
                            err
                        );
                    }
                }
                let ready_packets = rtp_unpacker.try_dump();
                if first_rtp_timestamp.is_none() && !ready_packets.is_empty() {
                    *first_rtp_timestamp = Some(ready_packets[0].get_timestamp());
                    // time to send audio/video configs
                    if let Some(fmtp) = fmtp {
                        match rtpmap.encoding_name.to_lowercase().as_str() {
                            "h264" => {
                                let h264_fmtp: RtpH264Fmtp = fmtp.params.parse()?;
                                let config: AvcDecoderConfigurationRecord =
                                    (&h264_fmtp).try_into()?;
                                tracing::debug!("make avc decoder configuration record from fmtp: {:#?}", config);
                                let h264_sequence_header = MediaFrame::VideoConfig {
                                    timestamp_nano: 0,
                                        config: Box::new(config.into()),
                                    };
                                    
                                    media_frame_sender.send(h264_sequence_header).await.map_err(|err| {
                                        tracing::error!("send h264 sequence header to stream center failed: {}", err);
                                        RtspServerError::IoError(io::Error::other(format!("channel send h264 sequence header to stream center failed: {}", err)))
                                    })?;
                                    tracing::info!("publish h264 video sequence header to stream center succeed");
                                }
                                "mpeg4-generic" => {
                                    let aac_fmtp: RtpMpeg4Fmtp = fmtp.params.parse()?;                                    
                                    let config: AudioSpecificConfig = (&aac_fmtp).try_into()?;
                                    tracing::debug!("make aac specific config from fmtp: {:#?}", config);
                                    let aac_sequence_header = MediaFrame::AudioConfig {
                                        timestamp_nano: 0,
                                        sound_info: (&config).try_into().map_err(|e| {
                                            RtspServerError::CodecParametersError(format!("convert aac specific config to sound info failed: {}", e))
                                        })?,
                                        config: Box::new(config.into())
                                    };
                                    media_frame_sender.send(aac_sequence_header).await.map_err(|err| {
                                        tracing::error!("send aac sequence header to stream center failed: {}", err);
                                        RtspServerError::IoError(io::Error::other(format!("channel send aac sequence header to stream center failed: {}", err)))
                                    })?;
                                    tracing::info!("publish aac audio sequence header to stream center succeed");
                                }
                                _ => {
                                    unimplemented!()
                                }
                            }
                        }
                    }
                    for packet in ready_packets {
                        match media_frame_sender.send(packet.to_media_frame(first_rtp_timestamp.unwrap(), rtp_clockrate)).await {
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
                Ok(())
            }).await,
        }
    }

    async fn process_commands(&mut self, span: &Span) -> RtspServerResult<()> {
        let command = self.rtsp_session_command_rx.try_recv();
        span.in_scope(async || match command {
            Err(TryRecvError::Closed) => {
                tracing::warn!("rtsp session command channel closed");
                Err(RtspServerError::IoError(io::Error::other(
                    "rtsp session command channel closed",
                )))
            }
            Err(TryRecvError::Lagged(skipped)) => {
                tracing::warn!("rtsp session command channel lagged, skipped {} messages", skipped);
                Ok(())
            }
            Err(_) => Ok(()),
            Ok(command) => match command {
                RtspSessionCommand::Start => Ok(()),
                RtspSessionCommand::Stop => {
                    tracing::info!("rtsp session is stopping");
                    self.rtp_session_command_tx
                        .send(RtpSessionCommand::Stop).await
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
                    .send(RtpSessionCommand::Rtp(packet)).await
                    .map_err(|err| {
                        tracing::error!("failed to send rtp packet: {:?}", err);
                        RtspServerError::IoError(io::Error::other(format!(
                            "failed to send rtp packet: {:?}",
                            err
                        )))
                    }),
                RtspSessionCommand::Rtcp(packet) => self
                    .rtp_session_command_tx
                    .send(RtpSessionCommand::Rtcp(packet)).await
                    .map_err(|err| {
                        tracing::error!("failed to send rtcp packet: {:?}", err);
                        RtspServerError::IoError(io::Error::other(format!(
                            "failed to send rtcp packet: {:?}",
                            err
                        )))
                    }),
            },
        }).await
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

    fn extract_bandwidth(media_description: &SDPMediaDescription) -> RtspServerResult<u64> {
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

    pub fn session_id(&self) -> &str {
        self.session_id.as_str()
    }

    pub fn media_type(&self) -> &SDPMediaType {
        &self.media_type
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
