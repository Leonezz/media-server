use crate::{
    errors::{RtspServerError, RtspServerResult},
    media_session::{RtspMediaSession, RtspSessionCommand},
    middleware::RtspMiddleware,
    rtsp_server_simple_response,
};
use chrono::TimeDelta;
use codec_common::audio::AudioConfig;
use futures::{SinkExt, StreamExt};
use num::ToPrimitive;
use rtp_formats::{
    codec::{
        h264::paramters::{RtpH264Fmtp, RtpH264FmtpBuilder, packetization_mode::PacketizationMode},
        mpeg4_generic::parameters::RtpMpeg4Fmtp,
    },
    payload_types::rtp_payload_type::{
        audio_get_rtp_clockrate, audio_get_rtp_encoding_name, get_audio_rtp_payload_type,
        get_video_rtp_payload_type, video_get_rtp_clockrate, video_get_rtp_encoding_name,
    },
};
use rtsp_formats::{
    RtspMessage, RtspMessageFramed,
    consts::{
        methods::{RTSP_METHODS, RtspMethod},
        status::RtspStatus,
    },
    errors::RtspMessageError,
    header::{
        RtspHeader,
        transport::{TransportHeader, TransportMode},
    },
    interleaved::RtspInterleavedPacket,
    request::RtspRequest,
    response::{RtspResponse, builder::RtspResponseBuilder},
    sdp_extension::attribute::RtspSDPControl,
};
use scopeguard::defer;
use sdp_formats::{
    attributes::{SDPAttribute, fmtp::FormatParameters, rtpmap::RtpMap},
    builder::{SdpBuilder, SdpMediaBuilder},
    session::{SDPAddrType, SDPMediaDescription, SDPMediaType, SDPNetType, Sdp},
};
use server_utils::{
    runtime_handle::{PlayHandle, PublishHandle, SessionRuntime},
    stream_properities::StreamProperties,
};
use std::{collections::HashMap, net::SocketAddr, pin::Pin, sync::Arc};
use stream_center::{
    errors::StreamCenterError,
    gop::MediaFrame,
    stream_center::StreamCenter,
    stream_source::{PlayProtocol, PublishProtocol, StreamIdentifier},
};
use tokio::sync::{RwLock, mpsc::UnboundedSender};
use tracing::Instrument;
use unified_io::{UnifiedIO, UnifiyStreamed};
use url::Url;
use uuid::Uuid;

#[derive(Debug)]
pub struct RtspMediaSessionHandler {
    pub(crate) peer_addr: SocketAddr,
    pub(crate) uri: Url,
    pub(crate) session_id: String,
    pub(crate) media_sdp: SDPMediaDescription,
    pub(crate) transport: TransportHeader,
}

pub struct RtspSession {
    stream_center_event_sender: UnboundedSender<stream_center::events::StreamCenterEvent>,
    io: UnifiyStreamed<RtspMessageFramed>,
    peer_addr: SocketAddr,
    sdp: Option<Sdp>,
    range: Option<String>,
    session_id: Option<String>,
    timeout_ms: u64,
    media_sessions: Arc<RwLock<HashMap<String, RtspMediaSessionHandler>>>,
    stream_properities: Option<StreamProperties>,
    runtime_handle: SessionRuntime,
    rtsp_command_tx: tokio::sync::broadcast::Sender<RtspSessionCommand>,
    middlewares: Vec<Box<dyn RtspMiddleware + Send>>,
}

impl RtspMiddleware for RtspSession {
    fn pre_request(&mut self, request: RtspRequest) -> RtspServerResult<RtspRequest> {
        self.middlewares
            .iter_mut()
            .try_fold(request, |req, mid| mid.pre_request(req))
    }

    fn pre_response(
        &mut self,
        request: &RtspRequest,
        response: RtspResponse,
    ) -> RtspServerResult<RtspResponse> {
        self.middlewares
            .iter_mut()
            .try_fold(response, |res, mid| mid.pre_response(request, res))
    }
}

impl RtspSession {
    pub fn new(
        stream_center_event_sender: UnboundedSender<stream_center::events::StreamCenterEvent>,
        io: Pin<Box<dyn UnifiedIO + Send>>,
        peer_addr: SocketAddr,
    ) -> Self {
        let (rtsp_command_tx, _) = tokio::sync::broadcast::channel(1000);
        Self {
            stream_center_event_sender,
            io: UnifiyStreamed::new(io, RtspMessageFramed),
            peer_addr,
            sdp: None,
            range: None,
            session_id: Default::default(),
            timeout_ms: 60_000,
            media_sessions: Arc::new(RwLock::new(HashMap::new())),
            stream_properities: Default::default(),
            runtime_handle: SessionRuntime::Unknown,
            rtsp_command_tx,
            middlewares: vec![],
        }
    }

    pub fn with_middleware(mut self, middleware: Box<dyn RtspMiddleware + Send>) -> Self {
        self.middlewares.push(middleware);
        self
    }

    pub async fn send_response(
        &mut self,
        request: &RtspRequest,
        response: RtspResponse,
    ) -> RtspServerResult<()> {
        let mut response = self.pre_response(request, response)?;
        if let Some(session_id) = self.session_id.as_ref()
            && !request.headers().contains(RtspHeader::Session)
        {
            response.headers_mut().push(RtspHeader::Session, session_id);
        }
        tracing::debug!("sending rtsp response: {:?}", response);
        self.io.send(RtspMessage::Response(response)).await?;
        Ok(())
    }

    async fn on_session_pre_exit(&mut self) {
        match self.rtsp_command_tx.send(RtspSessionCommand::Stop) {
            Ok(cnt) => {
                tracing::info!(
                    "send stop command to all media sessions succeed, {} receivers notified",
                    cnt
                );
            }
            Err(err) => {
                tracing::error!(
                    "got error when sending stop command to all media sessions: {}",
                    err
                );
            }
        }
        match &self.runtime_handle {
            SessionRuntime::Play(_) => {
                tracing::info!(
                    "play session is about to exit, session_id={:?}",
                    self.session_id,
                );
                self.unsubscribe_stream().await.unwrap_or_else(|err| {
                    tracing::error!("error while unsubscribe stream: {}", err);
                });
            }
            SessionRuntime::Publish(_) => {
                tracing::info!(
                    "publish session is about to exit, session_id={:?}",
                    self.session_id
                );
                self.unpublish_stream().await.unwrap_or_else(|err| {
                    tracing::error!("error while unpublish stream: {}", err);
                });
            }
            SessionRuntime::Unknown => {
                tracing::info!(
                    "unknown session is about to exit, session_id={:?}",
                    self.session_id
                );
            }
        }
    }

    pub async fn run(&mut self) -> RtspServerResult<()> {
        tracing::info!("rtsp session is running");
        loop {
            match self.read_rtsp_message().await {
                Ok(()) => {}
                Err(err) => {
                    tracing::error!("error while reading rtsp message: {}", err);
                    self.on_session_pre_exit().await;
                    return Err(err);
                }
            }
        }
    }

    pub async fn read_rtsp_message(&mut self) -> RtspServerResult<()> {
        match self.io.next().await {
            Some(Ok(message)) => {
                tracing::debug!("received rtsp message: {:?}", message);
                match message {
                    RtspMessage::Request(request) => {
                        tracing::debug!("handle rtsp request: {}", request);
                        let request_span = tracing::debug_span!(
                            "handle_request",
                            method = request.method().to_string(),
                            uri = request.uri().to_string(),
                            session_id = request.headers().get_unique(RtspHeader::Session),
                            cseq = request.headers().cseq(),
                        );
                        let request = request_span.in_scope(|| self.pre_request(request))?;

                        let response = if self.session_id.as_ref()
                            != request.headers().get_unique(RtspHeader::Session)
                        {
                            Ok(rtsp_server_simple_response(RtspStatus::SessionNotFound))
                        } else {
                            self.handle_request(&request).instrument(request_span).await
                        };
                        match response {
                            Ok(response) => {
                                tracing::info!("response: {}", response);
                                self.send_response(&request, response).await?
                            }
                            Err(RtspServerError::ParseStreamProperitiesFailed(err)) => {
                                tracing::error!(
                                    "error while parsing request url to stream properities: {}",
                                    err
                                );
                                self.send_response(
                                    &request,
                                    rtsp_server_simple_response(RtspStatus::BadRequest),
                                )
                                .await?
                            }
                            Err(RtspServerError::StreamCenterError(
                                StreamCenterError::StreamNotFound(s),
                            )) => {
                                tracing::error!("stream requested not found: {:?}", s);
                                self.send_response(
                                    &request,
                                    rtsp_server_simple_response(RtspStatus::NotFound),
                                )
                                .await?
                            }
                            Err(RtspServerError::StreamCenterError(
                                StreamCenterError::DuplicateStream(err),
                            )) => {
                                tracing::error!("stream already published: {:?}", err);
                                self.send_response(
                                    &request,
                                    rtsp_server_simple_response(RtspStatus::BadRequest),
                                )
                                .await?
                            }
                            Err(err) => {
                                tracing::error!("error while processing request: {}", err);
                                self.send_response(
                                    &request,
                                    rtsp_server_simple_response(RtspStatus::InternalServerError),
                                )
                                .await?
                            }
                        }
                    }
                    RtspMessage::Response(response) => {
                        self.on_rtsp_response(response).await?;
                    }
                    RtspMessage::Interleaved(interleaved) => {
                        self.on_rtsp_interleaved(interleaved).await?;
                    }
                }
            }
            Some(Err(RtspMessageError::Io(err)))
                if err.kind() == std::io::ErrorKind::WouldBlock => {}
            Some(Err(RtspMessageError::Io(err)))
                if err.kind() == std::io::ErrorKind::ConnectionReset =>
            {
                tracing::info!("connection reset by peer");
                return Ok(());
            }
            Some(Err(e)) => {
                tracing::error!("error receiving rtsp message: {:?}", e);
                return Err(RtspServerError::RtspMessageError(e));
            }
            None => {
                tracing::info!("connection reset");
                return Err(RtspServerError::GracefulExit);
            }
        }
        Ok(())
    }

    pub async fn on_rtsp_response(&mut self, response: RtspResponse) -> RtspServerResult<()> {
        tracing::debug!("received rtsp response: {:?}", response);

        Ok(())
    }

    pub async fn on_rtsp_interleaved(
        &mut self,
        interleaved: RtspInterleavedPacket,
    ) -> RtspServerResult<()> {
        tracing::debug!("received rtsp interleaved packet: {:?}", interleaved);

        Ok(())
    }

    fn session_pre_setup(
        &mut self,
        stream_properities: StreamProperties,
        is_publish: bool,
    ) -> Option<RtspResponse> {
        // TODO - maybe unpublish and re-publish
        if !matches!(self.runtime_handle, SessionRuntime::Unknown) {
            tracing::error!(
                "trying to {} to session that is already established for {}",
                if is_publish { "publish" } else { "play" },
                if self.runtime_handle.is_play() {
                    "play"
                } else {
                    "publish"
                }
            );
            return Some(rtsp_server_simple_response(
                RtspStatus::MethodNotValidInThisState,
            ));
        }

        if let Some(stream_prop) = self.stream_properities.as_ref()
            && (stream_prop.stream_name != stream_properities.stream_name
                || stream_prop.app != stream_properities.app)
        {
            tracing::error!(
                "trying to {} a different stream in {} session, old stream={:?}, new stream={:?}",
                if is_publish { "publish" } else { "play" },
                if self.runtime_handle.is_play() {
                    "play"
                } else {
                    "publish"
                },
                self.stream_properities,
                stream_properities
            );
            return Some(rtsp_server_simple_response(RtspStatus::SessionNotFound));
        }
        self.stream_properities = Some(stream_properities);
        None
    }

    async fn publish_stream(
        &mut self,
        request: &RtspRequest,
    ) -> RtspServerResult<Option<RtspResponse>> {
        let stream_prop: StreamProperties = request.uri().try_into()?;
        if let Some(res) = self.session_pre_setup(stream_prop, true) {
            return Ok(Some(res));
        }
        let media_sender = StreamCenter::publish(
            &self.stream_center_event_sender,
            PublishProtocol::RTSP,
            &StreamIdentifier {
                stream_name: self
                    .stream_properities
                    .as_ref()
                    .unwrap()
                    .stream_name
                    .clone(),
                app: self.stream_properities.as_ref().unwrap().app.clone(),
            },
            &self.stream_properities.as_ref().unwrap().stream_context,
        )
        .await;
        if let Err(err) = media_sender {
            tracing::error!("rtsp stream publish to stream center failed: {}", err);
            return Err(err.into());
        }
        self.runtime_handle = SessionRuntime::Publish(Arc::new(RwLock::new(PublishHandle {
            stream_data_producer: media_sender.unwrap(),
            no_data_since: None,
        })));
        tracing::info!("rtsp stream publish to stream center succeed");
        Ok(None)
    }

    async fn unpublish_stream(&mut self) -> RtspServerResult<()> {
        if let Some(stream_prop) = self.stream_properities.as_ref() {
            let unpublish_response = StreamCenter::unpublish(
                &self.stream_center_event_sender,
                &StreamIdentifier {
                    stream_name: stream_prop.stream_name.clone(),
                    app: stream_prop.app.clone(),
                },
            )
            .await;
            if let Err(err) = unpublish_response {
                tracing::error!("rtsp stream unpublish from stream center failed: {}", err);
                return Err(err.into());
            }
            tracing::info!("rtsp stream unpublish from stream center succeed");
        }
        Ok(())
    }

    async fn unsubscribe_stream(&mut self) -> RtspServerResult<()> {
        let play_handle = self.runtime_handle.get_play_handle();
        if play_handle.is_none() {
            return Ok(());
        }
        let play_id = play_handle.unwrap().read().await.play_id;
        if let Some(stream_prop) = self.stream_properities.as_ref() {
            let unsubscribe_response = StreamCenter::unsubscribe(
                &self.stream_center_event_sender,
                play_id,
                &StreamIdentifier {
                    stream_name: stream_prop.stream_name.clone(),
                    app: stream_prop.app.clone(),
                },
            )
            .await;
            if let Err(err) = unsubscribe_response {
                tracing::error!("rtsp unsubscribe from stream center failed: {}", err);
                return Err(err.into());
            }
            tracing::info!("rtsp unsubscribe from stream center succeed");
        }
        Ok(())
    }

    async fn subscribe_stream(
        &mut self,
        request: &RtspRequest,
    ) -> RtspServerResult<Option<RtspResponse>> {
        let stream_prop: StreamProperties = request.uri().try_into()?;
        if let Some(res) = self.session_pre_setup(stream_prop, false) {
            return Ok(Some(res));
        }

        let subscribe_response = StreamCenter::subscribe(
            &self.stream_center_event_sender,
            PlayProtocol::RTSP,
            &StreamIdentifier {
                stream_name: self
                    .stream_properities
                    .as_ref()
                    .unwrap()
                    .stream_name
                    .clone(),
                app: self.stream_properities.as_ref().unwrap().app.clone(),
            },
            &self.stream_properities.as_ref().unwrap().stream_context,
        )
        .await;

        if let Err(err) = subscribe_response {
            tracing::error!("rtsp subscribe from stream center failed: {}", err);
            return Err(err.into());
        }
        let subscribe_response = subscribe_response.unwrap();
        self.runtime_handle = SessionRuntime::Play(Arc::new(RwLock::new(PlayHandle {
            stream_data_consumer: subscribe_response.media_receiver,
            play_id: subscribe_response.subscribe_id,
            receive_audio: true,
            receive_video: true,
            buffer_length: None,
        })));

        Ok(None)
    }

    async fn new_play_session(
        &mut self,
        request: &RtspRequest,
        transport: &TransportHeader,
    ) -> RtspServerResult<RtspResponse> {
        tracing::debug!(
            "creating new rtsp play session with request: {}, transport: {}",
            request,
            transport
        );

        let sdp = self.sdp.as_ref().unwrap();
        let mut server_transport = transport.clone();
        let generated_session_id = Uuid::now_v7().to_string();
        let this_session_id = self
            .session_id
            .as_ref()
            .unwrap_or(&generated_session_id)
            .clone();
        let mut response_builder = RtspResponse::builder();

        let mut frame_distributors = vec![];
        for media in &sdp.media_description {
            let control = media.attributes.iter().find_map(|attr| {
                if let SDPAttribute::Trivial(attr) = attr
                    && attr.name == "control"
                {
                    RtspSDPControl::try_from(attr).ok()
                } else {
                    None
                }
            });
            if control.is_none() {
                tracing::warn!("media control attribute not found");
                continue;
            }
            let rtpmap = media.get_rtp_map();
            if rtpmap.is_none() {
                tracing::warn!("rtpmap attribute not found");
                continue;
            }
            let control = control.unwrap();
            let control_str = control.url_to_str();
            if !request.uri().path().contains(control_str.as_str()) {
                continue;
            }

            if let Some(session) = self.media_sessions.read().await.get(control_str.as_str()) {
                tracing::warn!("media session already exists: {:?}", session);
            }

            tracing::info!(
                "new rtsp media play session, session id: {}, uri: {}, control: {}, media_description: {:?}, transport: {}",
                this_session_id,
                request.uri(),
                control,
                media,
                transport,
            );

            let (media_frame_distributor_tx, media_frame_distributor_rx) =
                tokio::sync::mpsc::channel::<MediaFrame>(1000);
            frame_distributors.push((
                matches!(media.media_line.media_type, SDPMediaType::Video),
                media_frame_distributor_tx,
            ));
            let media_session = RtspMediaSession::new_play_session(
                self.peer_addr,
                request.uri().clone(),
                &control,
                media,
                &rtpmap.unwrap(),
                this_session_id.clone(),
                transport.clone(),
                self.rtsp_command_tx.subscribe(),
                media_frame_distributor_rx,
            )
            .await;
            if let Err(err) = media_session {
                if let RtspServerError::InvalidTransport(err) = err {
                    tracing::error!("transport: {} is invalid", err);
                    return Ok(rtsp_server_simple_response(
                        RtspStatus::UnsupportedTransport,
                    ));
                } else {
                    tracing::error!("error while create new media session: {}", err);
                    return Err(err);
                }
            }
            let mut media_session = media_session.unwrap();
            tracing::info!(
                "media session created for session id: {}, control: {}",
                this_session_id,
                control_str
            );

            server_transport
                .server_port
                .replace((media_session.local_rtp_port, media_session.local_rtcp_port));
            response_builder =
                response_builder.header(RtspHeader::Transport, format!("{}", server_transport));
            media_session.transport = server_transport.clone();
            tokio::task::spawn(async move {
                if let Err(err) = media_session.run().await {
                    tracing::error!("media session error: {:?}", err);
                } else {
                    tracing::info!("media session exited gracefully");
                }
            });

            self.media_sessions.write().await.insert(
                control_str,
                RtspMediaSessionHandler {
                    peer_addr: self.peer_addr,
                    uri: request.uri().clone(),
                    session_id: this_session_id.clone(),
                    media_sdp: media.clone(),
                    transport: transport.clone(),
                },
            );
            match media.media_line.media_type {
                SDPMediaType::Video => {}
                SDPMediaType::Audio => {}
                _ => {
                    tracing::warn!("unsupported media type: {:?}", media.media_line.media_type);
                    return Ok(rtsp_server_simple_response(RtspStatus::BadRequest));
                }
            }
        }

        if self.session_id.is_none()
            && let Some(response) = self.subscribe_stream(request).await?
        {
            return Ok(response);
        }
        let play_handle = self.runtime_handle.get_play_handle().unwrap().clone();
        let mut rtsp_command_receiver = self.rtsp_command_tx.subscribe();
        let rtsp_command_sender = self.rtsp_command_tx.clone();
        tokio::spawn(async move {
            defer!(let _ = rtsp_command_sender.send(RtspSessionCommand::Stop););
            loop {
                match rtsp_command_receiver.try_recv() {
                    Ok(RtspSessionCommand::Stop) => {
                        tracing::info!("play session received teardown command, exiting");
                        return;
                    }
                    Ok(_) => {}
                    Err(tokio::sync::broadcast::error::TryRecvError::Closed) => {
                        tracing::info!("play session command channel closed, exiting");
                        return;
                    }
                    Err(tokio::sync::broadcast::error::TryRecvError::Lagged(skipped)) => {
                        tracing::warn!(
                            "play session command channel lagged, skipped {} messages",
                            skipped
                        );
                    }
                    Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {}
                }
                let mut play_handle = play_handle.write().await;
                match play_handle.stream_data_consumer.recv().await {
                    Some(frame) => {
                        for (is_video, distributor) in &frame_distributors {
                            if (*is_video && frame.is_video() || !*is_video && frame.is_audio())
                                && let Err(err) = distributor.send(frame.clone()).await
                            {
                                tracing::error!("failed to distribute media frame: {}", err);
                                return;
                            }
                        }
                    }
                    None => {
                        tracing::info!("no more media frames, exiting");
                        return;
                    }
                }
            }
        });

        if self.session_id.is_none() {
            tracing::trace!("new publish session, session_id={}", this_session_id);
            self.session_id = Some(this_session_id);
        }
        let response = response_builder
            .header(RtspHeader::Session, format!(
                "{};timeout={}",
                self.session_id.as_ref().unwrap(),
                self.timeout_ms.checked_div(1000).unwrap())
            )
            .header(RtspHeader::AcceptRanges, "npt")
            .header(RtspHeader::MediaProperties, "Random Access: No-Seeking, Content Modifications: TimeProgressing, Retention: Time-Duration=0.0")
            .ok()
            .build()?;
        Ok(response)
    }
    async fn new_publish_session(
        &mut self,
        request: &RtspRequest,
        transport: &TransportHeader,
    ) -> RtspServerResult<RtspResponse> {
        tracing::debug!(
            "creating new rtsp publish session with request: {}, transport: {}",
            request,
            transport
        );
        if self.sdp.is_none() {
            tracing::error!("sdp is not set by now, unable to handle SETUP request");
            return Ok(rtsp_server_simple_response(RtspStatus::NotAcceptable));
        }

        if self.session_id.is_none()
            && let Some(response) = self.publish_stream(request).await?
        {
            return Ok(response);
        }
        let sdp = self.sdp.as_ref().unwrap();
        let mut server_transport = transport.clone();

        let generated_session_id = Uuid::now_v7().to_string();
        let this_session_id = self.session_id.as_ref().unwrap_or(&generated_session_id);
        let mut response_builder = RtspResponse::builder();
        for media in &sdp.media_description {
            let control = media.attributes.iter().find_map(|attr| {
                if let SDPAttribute::Trivial(attr) = attr
                    && attr.name == "control"
                {
                    RtspSDPControl::try_from(attr).ok()
                } else {
                    None
                }
            });
            if control.is_none() {
                tracing::warn!("media control attribute not found");
                continue;
            }
            let control = control.unwrap();
            let control_str = control.url_to_str();
            if !request.uri().path().contains(control_str.as_str()) {
                continue;
            }

            if let Some(session) = self.media_sessions.read().await.get(control_str.as_str()) {
                tracing::warn!("media session already exists: {:?}", session);
            }

            tracing::info!(
                "new rtsp media publish session, session id: {}, uri: {}, control: {}, media_description: {:?}, transport: {}",
                this_session_id,
                request.uri(),
                control,
                media,
                transport,
            );

            let media_session = RtspMediaSession::new_publish_session(
                self.peer_addr,
                request.uri().clone(),
                this_session_id.clone(),
                media.clone(),
                transport.clone(),
                self.rtsp_command_tx.subscribe(),
                self.runtime_handle
                    .get_publish_handle()
                    .unwrap()
                    .clone()
                    .read()
                    .await
                    .stream_data_producer
                    .clone(),
            )
            .await;
            if let Err(err) = media_session {
                if let RtspServerError::InvalidTransport(err) = err {
                    tracing::error!("transport: {} is invalid", err);
                    return Ok(rtsp_server_simple_response(
                        RtspStatus::UnsupportedTransport,
                    ));
                } else {
                    tracing::error!("error while create new media session: {}", err);
                    return Err(err);
                }
            }
            let mut media_session = media_session.unwrap();
            tracing::info!(
                "media session created for session id: {}, control: {}",
                this_session_id,
                control_str
            );

            server_transport
                .server_port
                .replace((media_session.local_rtp_port, media_session.local_rtcp_port));
            response_builder =
                response_builder.header(RtspHeader::Transport, format!("{}", server_transport));

            media_session.transport = server_transport.clone();
            tokio::task::spawn(async move {
                if let Err(err) = media_session.run().await {
                    tracing::error!("media session error: {:?}", err);
                } else {
                    tracing::info!("media session exited gracefully");
                }
            });
            self.media_sessions.write().await.insert(
                control_str,
                RtspMediaSessionHandler {
                    peer_addr: self.peer_addr,
                    uri: request.uri().clone(),
                    session_id: this_session_id.clone(),
                    media_sdp: media.clone(),
                    transport: transport.clone(),
                },
            );

            match media.media_line.media_type {
                SDPMediaType::Video => {}
                SDPMediaType::Audio => {}
                _ => {
                    tracing::warn!("unsupported media type: {:?}", media.media_line.media_type);
                    return Ok(rtsp_server_simple_response(RtspStatus::BadRequest));
                }
            }
        }

        if self.session_id.is_none() {
            tracing::trace!("new publish session, session_id={}", this_session_id);
            self.session_id = Some(this_session_id.clone());
        }
        let response = response_builder
            .header(RtspHeader::Session, format!(
                "{};timeout={}",
                self.session_id.as_ref().unwrap(),
                self.timeout_ms.checked_div(1000).unwrap())
            )
            .header(RtspHeader::AcceptRanges, "npt")
            .header(RtspHeader::MediaProperties, "Random Access: No-Seeking, Content Modifications: TimeProgressing, Retention: Time-Duration=0.0")
            .ok()
            .build()?;

        Ok(response)
    }
}

trait RtspRequestHandler {
    async fn handle_request(&mut self, request: &RtspRequest) -> RtspServerResult<RtspResponse> {
        // Handle the request here
        match request.method() {
            RtspMethod::Options => self.handle_options(request).await,
            RtspMethod::Describe => self.handle_describe(request).await,
            RtspMethod::Setup => self.handle_setup(request).await,
            RtspMethod::Play => self.handle_play(request).await,
            RtspMethod::Pause => self.handle_pause(request).await,
            RtspMethod::TearDown => self.handle_teardown(request).await,
            RtspMethod::GetParameter => self.handle_get_parameter(request).await,
            RtspMethod::PlayNotify => self.handle_play_notify(request).await,
            RtspMethod::SetParameter => self.handle_set_parameter(request).await,
            RtspMethod::Redirect => self.handle_redirect(request).await,
            RtspMethod::Announce => self.handle_announce(request).await,
            RtspMethod::Record => self.handle_record(request).await,
        }
    }
    async fn handle_options(&mut self, request: &RtspRequest) -> RtspServerResult<RtspResponse>;
    async fn handle_describe(&mut self, request: &RtspRequest) -> RtspServerResult<RtspResponse>;
    async fn handle_setup(&mut self, request: &RtspRequest) -> RtspServerResult<RtspResponse>;
    async fn handle_play(&mut self, request: &RtspRequest) -> RtspServerResult<RtspResponse>;
    async fn handle_pause(&mut self, request: &RtspRequest) -> RtspServerResult<RtspResponse>;
    async fn handle_teardown(&mut self, request: &RtspRequest) -> RtspServerResult<RtspResponse>;
    async fn handle_get_parameter(
        &mut self,
        request: &RtspRequest,
    ) -> RtspServerResult<RtspResponse>;
    async fn handle_set_parameter(
        &mut self,
        request: &RtspRequest,
    ) -> RtspServerResult<RtspResponse>;
    async fn handle_play_notify(&mut self, request: &RtspRequest)
    -> RtspServerResult<RtspResponse>;
    async fn handle_redirect(&mut self, request: &RtspRequest) -> RtspServerResult<RtspResponse>;
    async fn handle_announce(&mut self, request: &RtspRequest) -> RtspServerResult<RtspResponse>;
    async fn handle_record(&mut self, request: &RtspRequest) -> RtspServerResult<RtspResponse>;
}

impl RtspRequestHandler for RtspSession {
    async fn handle_options(&mut self, _request: &RtspRequest) -> RtspServerResult<RtspResponse> {
        let response = RtspResponse::builder()
            .status(RtspStatus::OK)
            .header(RtspHeader::Public, RTSP_METHODS.join(","))
            .build()?;
        Ok(response)
    }

    async fn handle_describe(&mut self, request: &RtspRequest) -> RtspServerResult<RtspResponse> {
        // Handle DESCRIBE request
        let stream_properities: StreamProperties = request.uri().try_into()?;
        let stream_id = StreamIdentifier {
            stream_name: stream_properities.stream_name,
            app: stream_properities.app,
        };
        let media_description =
            StreamCenter::describe(&self.stream_center_event_sender, &stream_id).await?;

        tracing::info!("media description: {:#?}", media_description);
        let mut sdp_builder = SdpBuilder::new()
            .version(0)
            .origin_user_name("-".to_string())
            .origin_session_id(0)
            .origin_session_version(0)
            .origin_net_type(SDPNetType::IN)
            .origin_addr_type(SDPAddrType::IP4)
            .origin_unicast_address("0.0.0.0".to_string())
            .session_name(format!("{}", media_description.stream_id))
            .attribute(SDPAttribute::Trivial((&RtspSDPControl::Asterisk).into()))
            .time_info(0, 0, vec![]);

        if media_description.has_audio
            && let Some(audio_config) = media_description.audio_conifg
        {
            let codec_id = (&audio_config).into();
            let payload_type = get_audio_rtp_payload_type(codec_id).unwrap();
            let mut audio_sdp = SdpMediaBuilder::new()
                .media_type(SDPMediaType::Audio)
                .port(0.into())
                .protocol(sdp_formats::session::SDPMediaProtocol::RtpAvp)
                .media_format(payload_type.to_string())
                .attribute(SDPAttribute::Trivial(
                    (&RtspSDPControl::Relative("control=audio".to_owned())).into(),
                ))
                .rtpmap(RtpMap {
                    payload_type,
                    encoding_name: audio_get_rtp_encoding_name(codec_id).unwrap().to_string(),
                    clock_rate: audio_get_rtp_clockrate(codec_id).unwrap().to_u64().unwrap(),
                    encoding_params: None,
                });
            match audio_config {
                AudioConfig::AAC(aac_config) => {
                    let audio_fmtp: RtpMpeg4Fmtp = (&aac_config).try_into()?;
                    let fmtp = FormatParameters {
                        fmt: payload_type,
                        params: format!("{}", audio_fmtp),
                    };
                    audio_sdp = audio_sdp.fmtp(fmtp);
                }
            }
            sdp_builder = sdp_builder.media_description(audio_sdp.build());
        }
        if media_description.has_video
            && let Some(video_config) = media_description.video_config
        {
            let codec_id = (&video_config).into();
            let payload_type = get_video_rtp_payload_type(codec_id).unwrap();
            let mut video_sdp = SdpMediaBuilder::new()
                .media_type(SDPMediaType::Video)
                .port(0.into())
                .protocol(sdp_formats::session::SDPMediaProtocol::RtpAvp)
                .media_format(payload_type.to_string())
                .attribute(SDPAttribute::Trivial(
                    (&RtspSDPControl::Relative("control=video".to_owned())).into(),
                ))
                .rtpmap(RtpMap {
                    payload_type,
                    encoding_name: video_get_rtp_encoding_name(codec_id).unwrap().to_string(),
                    clock_rate: video_get_rtp_clockrate(codec_id).unwrap().to_u64().unwrap(),
                    encoding_params: None,
                });
            match video_config {
                codec_common::video::VideoConfig::H264(h264_config) => {
                    let video_fmtp: RtpH264Fmtp = RtpH264FmtpBuilder::from(&h264_config)
                        .packetization_mode(PacketizationMode::NonInterleaved)
                        .build();
                    let fmtp = FormatParameters {
                        fmt: payload_type,
                        params: format!("{}", video_fmtp),
                    };
                    video_sdp = video_sdp.fmtp(fmtp);
                }
            }
            sdp_builder = sdp_builder.media_description(video_sdp.build());
        }

        let sdp = sdp_builder.build();
        let sdp_str = sdp.to_string();
        self.sdp = Some(sdp);
        let response = RtspResponseBuilder::new()
            .header(RtspHeader::ContentType, "application/sdp")
            .header(RtspHeader::ContentBase, request.uri().as_str())
            .header(
                RtspHeader::Expires,
                chrono::Utc::now()
                    .checked_add_signed(TimeDelta::minutes(1))
                    .unwrap()
                    .to_rfc2822(),
            )
            .body(sdp_str)
            .status(RtspStatus::OK)
            .build()?;

        Ok(response)
    }

    async fn handle_setup(&mut self, request: &RtspRequest) -> RtspServerResult<RtspResponse> {
        let transport = request.headers().transport();
        if transport.is_none() {
            tracing::error!("transport header not found");
            return Ok(rtsp_server_simple_response(
                RtspStatus::UnsupportedTransport,
            ));
        }
        let transport = transport.unwrap();
        tracing::debug!("got SETUP request with transport: {:?}", &transport);

        let transport_mode = if transport.mode.is_empty() {
            &TransportMode::Play
        } else {
            &transport.mode[0]
        };

        match transport_mode {
            TransportMode::Play => self.new_play_session(request, &transport).await,
            TransportMode::Record => self.new_publish_session(request, &transport).await,
            TransportMode::Other(mode) => {
                tracing::error!("unknow transport mode in SETUP method: {}", mode);
                Ok(rtsp_server_simple_response(
                    RtspStatus::UnsupportedTransport,
                ))
            }
        }
    }

    async fn handle_play(&mut self, _request: &RtspRequest) -> RtspServerResult<RtspResponse> {
        // Handle PLAY request
        Ok(rtsp_server_simple_response(RtspStatus::OK))
    }

    async fn handle_pause(&mut self, _request: &RtspRequest) -> RtspServerResult<RtspResponse> {
        // Handle PAUSE request
        todo!()
    }

    async fn handle_teardown(&mut self, request: &RtspRequest) -> RtspServerResult<RtspResponse> {
        let session_id = request.headers().get_unique(RtspHeader::Session);
        if session_id.is_none() {
            return Ok(rtsp_server_simple_response(RtspStatus::BadRequest));
        }
        let session_id = session_id.unwrap();
        if self.session_id.is_none() || self.session_id.as_ref().unwrap().ne(session_id) {
            return Ok(rtsp_server_simple_response(RtspStatus::NotFound));
        }

        {
            tracing::info!("got teardown request, about to close session");
            self.on_session_pre_exit().await;
        }
        self.media_sessions.write().await.clear();
        self.session_id = None;
        self.sdp = None;
        self.range = None;

        Ok(rtsp_server_simple_response(RtspStatus::OK))
    }

    async fn handle_get_parameter(
        &mut self,
        request: &RtspRequest,
    ) -> RtspServerResult<RtspResponse> {
        tracing::debug!("get prarameter request: {}", request);
        Ok(rtsp_server_simple_response(RtspStatus::OK))
    }

    async fn handle_set_parameter(
        &mut self,
        _request: &RtspRequest,
    ) -> RtspServerResult<RtspResponse> {
        // Handle SET_PARAMETER request
        todo!()
    }

    async fn handle_play_notify(
        &mut self,
        _request: &RtspRequest,
    ) -> RtspServerResult<RtspResponse> {
        // Handle PLAY_NOTIFY request
        todo!()
    }

    async fn handle_redirect(&mut self, _request: &RtspRequest) -> RtspServerResult<RtspResponse> {
        // Handle REDIRECT request
        todo!()
    }

    async fn handle_announce(&mut self, request: &RtspRequest) -> RtspServerResult<RtspResponse> {
        let content_type = request.headers().get_unique(RtspHeader::ContentType);
        if content_type.is_none() || content_type.unwrap() != "application/sdp" {
            tracing::warn!(
                "announce content type is not application/sdp, got: {}",
                content_type.unwrap_or(&"None".to_owned())
            );
            return Ok(rtsp_server_simple_response(
                RtspStatus::UnsupportedMediaType,
            ));
        }

        let body = request.body().map(|v| v.parse::<Sdp>());

        if let Some(Ok(sdp)) = body {
            tracing::debug!("received SDP: {:?}", &sdp);
            self.sdp.replace(sdp);
        }

        Ok(rtsp_server_simple_response(RtspStatus::OK))
    }

    async fn handle_record(&mut self, request: &RtspRequest) -> RtspServerResult<RtspResponse> {
        if let Some(range) = request.headers().get_unique(RtspHeader::Range) {
            self.range.replace(range.to_owned());
        }

        let mut response = RtspResponse::builder().status(RtspStatus::OK);
        if let Some(range) = &self.range {
            response = response.header(RtspHeader::Range, range);
        }
        Ok(response.build()?)
    }
}
