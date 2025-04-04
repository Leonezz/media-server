use futures::{SinkExt, StreamExt};
use rtsp_formats::{
    RtspMessage, RtspMessageFramed,
    consts::{
        methods::{RTSP_METHODS, RtspMethod},
        status::RtspStatus,
    },
    errors::RtspMessageError,
    header::RtspHeader,
    interleaved::RtspInterleavedPacket,
    request::RtspRequest,
    response::RtspResponse,
};
use sdp_formats::session::SessionDescription;
use std::pin::Pin;
use tokio_util::codec::Framed;
use unified_io::UnifiedIO;

use crate::{
    SERVER_AGENT,
    errors::{RtspServerError, RtspServerResult},
};

#[derive(Debug)]
pub struct RtspSession {
    io: Framed<Pin<Box<dyn UnifiedIO + Send>>, RtspMessageFramed>,
}

impl RtspSession {
    pub fn new(io: Pin<Box<dyn UnifiedIO + Send>>) -> Self {
        Self {
            io: Framed::new(io, RtspMessageFramed),
        }
    }

    pub async fn run(&mut self) -> RtspServerResult<()> {
        tracing::info!("rtsp session is running");
        loop {
            match self.io.next().await {
                Some(Ok(message)) => {
                    tracing::debug!("received rtsp message: \n{}", message);
                    match message {
                        RtspMessage::Request(request) => {
                            self.handle_request(request).await?;
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
                    if err.kind() == std::io::ErrorKind::WouldBlock =>
                {
                    continue;
                }
                Some(Err(e)) => {
                    tracing::error!("error receiving rtsp message: {:?}", e);
                    return Err(RtspServerError::RtspMessageError(e));
                }
                None => {
                    continue;
                }
            }
        }
    }

    pub async fn on_rtsp_response(&mut self, response: RtspResponse) -> RtspServerResult<()> {
        tracing::debug!("received rtsp response: \n{}", response);

        Ok(())
    }

    pub async fn on_rtsp_interleaved(
        &mut self,
        interleaved: RtspInterleavedPacket,
    ) -> RtspServerResult<()> {
        tracing::debug!("received rtsp interleaved packet: \n{:?}", interleaved);

        Ok(())
    }
}

trait RtspRequestHandler {
    async fn handle_request(&mut self, request: RtspRequest) -> RtspServerResult<()> {
        tracing::debug!("handle rtsp request: \n{}", request);
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
}

impl RtspRequestHandler for RtspSession {
    async fn handle_options(&mut self, request: RtspRequest) -> RtspServerResult<()> {
        tracing::debug!("handling OPTIONS request");
        let seq = request.headers().cseq().unwrap_or(0);
        let response = RtspResponse::builder()
            .status(RtspStatus::OK)
            .version(request.version().clone())
            .header(RtspHeader::CSeq, seq.to_string())
            .header(RtspHeader::UserAgent, SERVER_AGENT)
            .header(RtspHeader::Public, RTSP_METHODS.join(","))
            .build()?;
        tracing::debug!("sending OPTIONS response: \n{}", response);
        self.io.send(RtspMessage::Response(response)).await?;
        Ok(())
    }

    async fn handle_describe(&mut self, _request: RtspRequest) -> RtspServerResult<()> {
        // Handle DESCRIBE request
        Ok(())
    }

    async fn handle_setup(&mut self, request: RtspRequest) -> RtspServerResult<()> {
        tracing::debug!("handling SETUP request");
        let seq = request.headers().cseq();
        let transport = request.headers().transport();
        tracing::debug!(
            "got SETUP request with seq: {:?}, transport: {:?}",
            seq,
            transport
        );
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

    async fn handle_teardown(&mut self, _request: RtspRequest) -> RtspServerResult<()> {
        // Handle TEARDOWN request
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
        tracing::debug!("handling ANNOUNCE request");

        let body = request.body().map(|v| v.parse::<SessionDescription>());

        if let Some(Ok(sdp)) = body {
            tracing::debug!("received SDP: \n{}", sdp);
        }

        let seq = request.headers().cseq().unwrap_or(0);
        let response = RtspResponse::builder()
            .status(RtspStatus::OK)
            .version(request.version().clone())
            .header(RtspHeader::CSeq, seq.to_string())
            .header(RtspHeader::UserAgent, SERVER_AGENT)
            .build()?;
        tracing::debug!("sending ANNOUNCE response: \n{}", response);
        self.io.send(RtspMessage::Response(response)).await?;
        Ok(())
    }
}
