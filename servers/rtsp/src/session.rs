use futures::{SinkExt, StreamExt};
use rtsp_formats::{
    RtspMessage, RtspMessageFramed,
    consts::{
        methods::{RTSP_METHODS, RtspMethod},
        status::RtspStatus,
    },
    errors::RtspMessageError,
    header::{RtspHeader, transport::TransportHeader},
    interleaved::RtspInterleavedPacket,
    request::RtspRequest,
    response::RtspResponse,
    sdp_extension::attribute::RtspSDPControl,
};
use sdp_formats::{
    attributes::SDPAttribute,
    session::{SDPMediaDescription, SDPMediaType, Sdp},
};
use server_utils::{
    runtime_handle::{PublishHandle, SessionRuntime},
    stream_properities::StreamProperties,
};
use std::{collections::HashMap, net::SocketAddr, pin::Pin, str::FromStr, sync::Arc};
use stream_center::{
    errors::StreamCenterError,
    stream_center::StreamCenter,
    stream_source::{PublishProtocol, StreamIdentifier},
};
use tokio::sync::{RwLock, mpsc::UnboundedSender};
use tracing::Instrument;
use unified_io::{UnifiedIO, UnifiyStreamed};
use url::Url;
use uuid::Uuid;

use crate::{
    errors::{RtspServerError, RtspServerResult},
    media_session::{RtspMediaSession, RtspSessionCommand},
    middleware::RtspMiddleware,
    rtsp_server_simple_response,
};

#[derive(Debug)]
pub struct RtspMediaSessionHandler {
    pub(crate) peer_addr: SocketAddr,
    pub(crate) uri: Url,
    pub(crate) session_id: Uuid,
    pub(crate) media_description: SDPMediaDescription,
    pub(crate) transport: TransportHeader,
    pub(crate) rtsp_command_tx: UnboundedSender<RtspSessionCommand>,
    media_session_thread: tokio::task::JoinHandle<()>,
}

pub struct RtspSession {
    stream_center_event_sender: UnboundedSender<stream_center::events::StreamCenterEvent>,
    io: UnifiyStreamed<RtspMessageFramed>,
    peer_addr: SocketAddr,
    sdp: Option<Sdp>,
    range: Option<String>,
    session_id: Option<Uuid>,
    timeout_ms: u64,
    media_sessions: Arc<RwLock<HashMap<String, RtspMediaSessionHandler>>>,
    stream_properities: StreamProperties,
    runtime_handle: SessionRuntime,
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
        Self {
            stream_center_event_sender,
            io: UnifiyStreamed::new(io, RtspMessageFramed),
            peer_addr,
            sdp: None,
            range: None,
            session_id: None,
            timeout_ms: 60_000,
            media_sessions: Arc::new(RwLock::new(HashMap::new())),
            stream_properities: Default::default(),
            runtime_handle: SessionRuntime::Unknown,
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
        let response = self.pre_response(request, response)?;
        tracing::debug!("sending rtsp response: {:?}", response);
        self.io.send(RtspMessage::Response(response)).await?;
        Ok(())
    }

    pub async fn run(&mut self) -> RtspServerResult<()> {
        tracing::info!("rtsp session is running");
        loop {
            self.read_rtsp_message().await?;
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
                        let response = self.handle_request(&request).instrument(request_span).await;
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
            None => {}
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

    async fn publish_stream(
        &mut self,
        request: &RtspRequest,
    ) -> RtspServerResult<Option<RtspResponse>> {
        let stream_properities: StreamProperties = request.uri().try_into()?;
        // TODO - maybe unpublish and re-publish
        if self.runtime_handle.is_play() {
            tracing::error!("trying to publish to session that is already established for playing");
            return Ok(Some(rtsp_server_simple_response(RtspStatus::BadRequest)));
        }

        if self.runtime_handle.is_publish()
            && (self.stream_properities.stream_name != stream_properities.stream_name
                || self.stream_properities.app.as_str() != stream_properities.app)
        {
            tracing::error!(
                "trying to publish a different stream to already published session, old stream={:?}, new stream={:?}",
                self.stream_properities,
                stream_properities
            );
            return Ok(Some(rtsp_server_simple_response(RtspStatus::BadRequest)));
        }
        self.stream_properities = stream_properities;
        if self.runtime_handle.is_publish() {
            tracing::info!(
                "stream {:?} already published, skip",
                self.stream_properities
            );
            return Ok(None);
        }
        let media_sender = StreamCenter::publish(
            &self.stream_center_event_sender,
            PublishProtocol::RTSP,
            &StreamIdentifier {
                stream_name: self.stream_properities.stream_name.clone(),
                app: self.stream_properities.app.clone(),
            },
            &self.stream_properities.stream_context,
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
        todo!()
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

        if self.sdp.is_none() {
            tracing::error!("sdp is not set by now, unable to handle SETUP request");
            return Ok(rtsp_server_simple_response(RtspStatus::NotAcceptable));
        }

        let session_id = request
            .headers()
            .get_unique(RtspHeader::Session)
            .map(|v| Uuid::from_str(v).unwrap_or(Uuid::now_v7()))
            .unwrap_or(Uuid::now_v7());
        if let Some(old_session_id) = self.session_id
            && old_session_id != session_id
        {
            tracing::warn!(
                "session id mismatch: old: {}, new: {}",
                old_session_id,
                session_id
            );
            return Ok(rtsp_server_simple_response(RtspStatus::SessionNotFound));
        }
        self.session_id.replace(session_id);

        if let Some(response) = self.publish_stream(request).await? {
            return Ok(response);
        }

        if !self.runtime_handle.is_publish() {
            tracing::error!(
                "media frame sender not set, which means stream not published to stream center, unable to distribute data"
            );
            return Ok(rtsp_server_simple_response(RtspStatus::NotFound));
        }
        let sdp = self.sdp.as_ref().unwrap();
        let mut server_transport = transport.clone();
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

            let (rtsp_command_tx, rtsp_command_rx) =
                tokio::sync::mpsc::unbounded_channel::<RtspSessionCommand>();

            tracing::info!(
                "new rtsp media session, session id: {}, uri: {}, control: {}, media_description: {:?}, transport: {}",
                session_id,
                request.uri(),
                control,
                media,
                transport,
            );
            let media_session = RtspMediaSession::new(
                self.peer_addr,
                request.uri().clone(),
                session_id,
                media.clone(),
                transport.clone(),
                rtsp_command_rx,
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
                session_id,
                control_str
            );

            server_transport
                .server_port
                .replace((media_session.local_rtp_port, media_session.local_rtcp_port));
            response_builder =
                response_builder.header(RtspHeader::Transport, format!("{}", server_transport));

            media_session.transport = server_transport.clone();
            let media_session_handler = tokio::task::spawn(async move {
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
                    session_id,
                    media_description: media.clone(),
                    transport: transport.clone(),
                    rtsp_command_tx,
                    media_session_thread: media_session_handler,
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
            tracing::warn!("session id is not set");
            return Ok(rtsp_server_simple_response(RtspStatus::BadRequest));
        }

        let response = response_builder
            .header(RtspHeader::Session, format!("{};timeout={}", self.session_id.unwrap(), self.timeout_ms.checked_div(1000).unwrap()))
            .header(RtspHeader::AcceptRanges, "npt")
            .header(RtspHeader::Session, self.session_id.unwrap())
            .header(RtspHeader::MediaProperties, "Random Access: No-Seeking, Content Modifications: TimeProgressing, Retention: Time-Duration=0.0")
            .status(RtspStatus::OK)
            .build()?;
        Ok(response)
    }

    async fn handle_play(&mut self, _request: &RtspRequest) -> RtspServerResult<RtspResponse> {
        // Handle PLAY request
        todo!()
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
        if self.session_id.is_none() || self.session_id.unwrap().to_string().ne(session_id) {
            return Ok(rtsp_server_simple_response(RtspStatus::NotFound));
        }

        let span = tracing::debug_span!("teardown", session_id = session_id);
        span.in_scope(async || {
            tracing::info!("got teardown request, about to close session");
            self.media_sessions
                .read()
                .await
                .iter()
                .for_each(|(control, handle)| {
                    tracing::info!(
                        "closing control: {}, peer: {}, uri: {}",
                        control,
                        handle.peer_addr,
                        handle.uri
                    );
                    match handle.rtsp_command_tx.send(RtspSessionCommand::Stop) {
                        Ok(()) => {
                            tracing::info!(
                                "send stop command to media session succeed, control={}",
                                control
                            );
                        }
                        Err(err) => {
                            tracing::error!(
                                "got error when sending stop command to media session: {}",
                                err
                            );
                        }
                    }
                });
        })
        .await;

        self.media_sessions.write().await.clear();
        self.session_id = None;
        self.sdp = None;
        self.range = None;

        Ok(rtsp_server_simple_response(RtspStatus::OK))
    }

    async fn handle_get_parameter(
        &mut self,
        _request: &RtspRequest,
    ) -> RtspServerResult<RtspResponse> {
        // Handle GET_PARAMETER request
        todo!()
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
        let parse_result = request.uri().try_into();
        match parse_result {
            Ok(result) => {
                self.stream_properities = result;
            }
            Err(err) => {
                tracing::error!(
                    "parse stream properities from setup request failed: {}",
                    err
                );
                return Err(err.into());
            }
        }
        tracing::info!("parsed stream properities: {:?}", self.stream_properities);

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
        if let Some(session_id) = self.session_id {
            response = response.header(RtspHeader::Session, session_id.to_string());
        }
        if let Some(range) = &self.range {
            response = response.header(RtspHeader::Range, range);
        }
        Ok(response.build()?)
    }
}
