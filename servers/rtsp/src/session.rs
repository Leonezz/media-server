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
    session::{SDPMediaDescription, SDPMediaType, SessionDescription},
};
use server_utils::stream_properities::StreamProperties;
use std::{collections::HashMap, net::SocketAddr, pin::Pin, str::FromStr, sync::Arc};
use stream_center::{
    gop::MediaFrame,
    stream_center::StreamCenter,
    stream_source::{PublishProtocol, StreamIdentifier, StreamType},
};
use tokio::sync::{
    RwLock,
    mpsc::{Sender, UnboundedSender},
};
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
    sdp: Option<SessionDescription>,
    range: Option<String>,
    session_id: Option<Uuid>,
    media_sessions: Arc<RwLock<HashMap<String, RtspMediaSessionHandler>>>,
    stream_properities: StreamProperties,
    middlewares: Vec<Box<dyn RtspMiddleware + Send>>,
    media_frame_sender: Option<Sender<MediaFrame>>,
}

impl RtspMiddleware for RtspSession {
    fn pre_request(&self, request: RtspRequest) -> RtspServerResult<RtspRequest> {
        self.middlewares
            .iter()
            .try_fold(request, |req, mid| mid.pre_request(req))
    }

    fn pre_response(
        &self,
        request: &RtspRequest,
        response: RtspResponse,
    ) -> RtspServerResult<RtspResponse> {
        self.middlewares
            .iter()
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
            media_sessions: Arc::new(RwLock::new(HashMap::new())),
            middlewares: Vec::new(),
            stream_properities: Default::default(),
            media_frame_sender: None,
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
                        self.handle_request(request)
                            .instrument(request_span)
                            .await?;
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
}

trait RtspRequestHandler {
    async fn handle_request(&mut self, request: RtspRequest) -> RtspServerResult<()> {
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
    async fn handle_options(&mut self, request: RtspRequest) -> RtspServerResult<()>;
    async fn handle_describe(&mut self, request: RtspRequest) -> RtspServerResult<()>;
    async fn handle_setup(&mut self, request: RtspRequest) -> RtspServerResult<()>;
    async fn handle_play(&mut self, request: RtspRequest) -> RtspServerResult<()>;
    async fn handle_pause(&mut self, request: RtspRequest) -> RtspServerResult<()>;
    async fn handle_teardown(&mut self, request: RtspRequest) -> RtspServerResult<()>;
    async fn handle_get_parameter(&mut self, request: RtspRequest) -> RtspServerResult<()>;
    async fn handle_set_parameter(&mut self, request: RtspRequest) -> RtspServerResult<()>;
    async fn handle_play_notify(&mut self, request: RtspRequest) -> RtspServerResult<()>;
    async fn handle_redirect(&mut self, request: RtspRequest) -> RtspServerResult<()>;
    async fn handle_announce(&mut self, request: RtspRequest) -> RtspServerResult<()>;
    async fn handle_record(&mut self, request: RtspRequest) -> RtspServerResult<()>;
}

impl RtspRequestHandler for RtspSession {
    async fn handle_options(&mut self, request: RtspRequest) -> RtspServerResult<()> {
        let response = RtspResponse::builder()
            .status(RtspStatus::OK)
            .header(RtspHeader::Public, RTSP_METHODS.join(","))
            .build()?;
        tracing::debug!("sending OPTIONS response: {:?}", response);
        self.send_response(&request, response).await?;
        Ok(())
    }

    async fn handle_describe(&mut self, _request: RtspRequest) -> RtspServerResult<()> {
        // Handle DESCRIBE request
        Ok(())
    }

    async fn handle_setup(&mut self, request: RtspRequest) -> RtspServerResult<()> {
        let transport = request.headers().transport();
        if transport.is_none() {
            tracing::error!("transport header not found");
            self.send_response(
                &request,
                rtsp_server_simple_response(RtspStatus::BadRequest),
            )
            .await?;
            return Ok(());
        }
        let transport = transport.unwrap();
        tracing::debug!("got SETUP request with transport: {:?}", &transport);

        if self.sdp.is_none() {
            tracing::error!("sdp is not set by now, unable to handle SETUP request");
            self.send_response(
                &request,
                rtsp_server_simple_response(RtspStatus::BadRequest),
            )
            .await?;
            return Ok(());
        }

        if self.media_frame_sender.is_none() {
            tracing::error!(
                "media frame sender not set, which means stream not published to stream center, unable to distribute data"
            );
            self.send_response(&request, rtsp_server_simple_response(RtspStatus::NotFound))
                .await?;
            return Ok(());
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
                self.send_response(
                    &request,
                    rtsp_server_simple_response(RtspStatus::SessionNotFound),
                )
                .await?;
                return Ok(());
            }

            self.session_id.replace(session_id);

            tracing::info!("session created, session id: {}", session_id);

            if let Some(session) = self.media_sessions.read().await.get(control_str.as_str()) {
                tracing::debug!("media session already exists: {:?}", session);
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
            if let Ok(mut media_session) = RtspMediaSession::new(
                self.peer_addr,
                request.uri().clone(),
                session_id,
                media.clone(),
                transport.clone(),
                rtsp_command_rx,
                self.media_frame_sender.as_ref().unwrap().clone(),
            )
            .await
            {
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
            } else {
                tracing::warn!("failed to create media session");
                self.send_response(
                    &request,
                    rtsp_server_simple_response(RtspStatus::InternalServerError),
                )
                .await?;
                return Ok(());
            }

            match media.media_line.media_type {
                SDPMediaType::Video => {}
                SDPMediaType::Audio => {}
                _ => {
                    tracing::warn!("unsupported media type: {:?}", media.media_line.media_type);
                    self.send_response(
                        &request,
                        rtsp_server_simple_response(RtspStatus::NotImplemented),
                    )
                    .await?;
                    return Ok(());
                }
            }
        }
        if self.session_id.is_none() {
            tracing::warn!("session id is not set");
            self.send_response(
                &request,
                rtsp_server_simple_response(RtspStatus::BadRequest),
            )
            .await?;
            return Ok(());
        }

        let response = response_builder
            .header(RtspHeader::Session, self.session_id.unwrap().to_string())
            .status(RtspStatus::OK)
            .build()?;
        self.send_response(&request, response).await?;
        Ok(())
    }

    async fn handle_play(&mut self, _request: RtspRequest) -> RtspServerResult<()> {
        // Handle PLAY request
        Ok(())
    }

    async fn handle_pause(&mut self, _request: RtspRequest) -> RtspServerResult<()> {
        // Handle PAUSE request
        Ok(())
    }

    async fn handle_teardown(&mut self, request: RtspRequest) -> RtspServerResult<()> {
        let session_id = request.headers().get_unique(RtspHeader::Session);
        if session_id.is_none() {
            self.send_response(
                &request,
                rtsp_server_simple_response(RtspStatus::BadRequest),
            )
            .await?;
            return Ok(());
        }
        let session_id = session_id.unwrap();
        if self.session_id.is_none() || self.session_id.unwrap().to_string().ne(session_id) {
            self.send_response(&request, rtsp_server_simple_response(RtspStatus::NotFound))
                .await?;
            return Ok(());
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

        self.send_response(&request, rtsp_server_simple_response(RtspStatus::OK))
            .await?;
        Ok(())
    }

    async fn handle_get_parameter(&mut self, _request: RtspRequest) -> RtspServerResult<()> {
        // Handle GET_PARAMETER request
        Ok(())
    }

    async fn handle_set_parameter(&mut self, _request: RtspRequest) -> RtspServerResult<()> {
        // Handle SET_PARAMETER request
        Ok(())
    }

    async fn handle_play_notify(&mut self, _request: RtspRequest) -> RtspServerResult<()> {
        // Handle PLAY_NOTIFY request
        Ok(())
    }

    async fn handle_redirect(&mut self, _request: RtspRequest) -> RtspServerResult<()> {
        // Handle REDIRECT request
        Ok(())
    }

    async fn handle_announce(&mut self, request: RtspRequest) -> RtspServerResult<()> {
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
                self.send_response(
                    &request,
                    rtsp_server_simple_response(RtspStatus::BadRequest),
                )
                .await?;
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
            self.send_response(
                &request,
                rtsp_server_simple_response(RtspStatus::UnsupportedMediaType),
            )
            .await?;

            return Ok(());
        }

        let body = request.body().map(|v| v.parse::<SessionDescription>());

        if let Some(Ok(sdp)) = body {
            tracing::debug!("received SDP: {:?}", &sdp);
            self.sdp.replace(sdp);
        }

        // TODO - maybe unpublish and re-publish
        if self.media_frame_sender.is_some() {
            tracing::error!("trying to publish to stream center while already published");
            self.send_response(
                &request,
                rtsp_server_simple_response(RtspStatus::BadRequest),
            )
            .await?;
            return Ok(());
        }

        let media_sender = StreamCenter::publish(
            &self.stream_center_event_sender,
            StreamType::Live,
            PublishProtocol::RTSP,
            &StreamIdentifier {
                stream_name: self.stream_properities.stream_name.clone(),
                app: self.stream_properities.app.clone(),
            },
            &self.stream_properities.stream_context,
        )
        .await;
        if media_sender.is_err() {
            let err = media_sender.unwrap_err();
            tracing::error!("rtsp stream publish to stream center failed: {}", err);
            self.send_response(
                &request,
                rtsp_server_simple_response(RtspStatus::InternalServerError),
            )
            .await?;
            return Err(err.into());
        }
        self.media_frame_sender = Some(media_sender.unwrap());
        tracing::info!("rtsp stream publish to stream center succeed");

        let response = rtsp_server_simple_response(RtspStatus::OK);
        self.send_response(&request, response).await?;
        Ok(())
    }

    async fn handle_record(&mut self, request: RtspRequest) -> RtspServerResult<()> {
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
        self.send_response(&request, response.build()?).await?;
        Ok(())
    }
}
