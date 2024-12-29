use std::{collections::HashMap, convert::Infallible, pin::Pin};

use http_body_util::StreamBody;
use hyper::{
    Method, Request, Response, StatusCode,
    body::{Bytes, Frame, Incoming},
    header,
    server::conn::http1,
    service::Service,
};
use hyper_util::rt::TokioIo;
use stream_center::events::StreamCenterEvent;
use tokio::sync::mpsc::{self};
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::{
    config::{HttpFlvServerConfig, HttpFlvSessionConfig},
    errors::HttpFlvServerResult,
    session::{HttpFlvSession, StreamProperties},
};

#[derive(Debug, Clone)]
pub struct HttpFlvServer {
    config: HttpFlvServerConfig,
    stream_center_event_sender: mpsc::UnboundedSender<StreamCenterEvent>,
}

impl HttpFlvServer {
    pub fn new(
        config: &HttpFlvServerConfig,
        stream_center_event_sender: mpsc::UnboundedSender<StreamCenterEvent>,
    ) -> Self {
        Self {
            config: config.clone(),
            stream_center_event_sender,
        }
    }

    pub async fn run(&mut self) -> HttpFlvServerResult<()> {
        tracing::info!("http flv server is running: {:?}", self.config);
        let listener =
            tokio::net::TcpListener::bind((self.config.ip.as_str(), self.config.port)).await?;
        loop {
            let (tcp_stream, addr) = listener.accept().await?;
            let peer_addr = tcp_stream.peer_addr();
            tracing::info!(
                "got new httpflv connection, addr: {}, peer addr: {:?}",
                addr,
                peer_addr
            );

            let tokio_io = TokioIo::new(tcp_stream);

            let service = self.clone();
            tokio::spawn(async move {
                let _ = http1::Builder::new()
                    .serve_connection(tokio_io, service)
                    .await;
            });
        }
    }
}

impl Service<Request<Incoming>> for HttpFlvServer {
    type Error = Infallible;
    type Response = Response<StreamBody<UnboundedReceiverStream<Result<Frame<Bytes>, Infallible>>>>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        let (response_sender, response_receiver) =
            mpsc::unbounded_channel::<Result<Frame<Bytes>, Infallible>>();
        let response_stream: StreamBody<UnboundedReceiverStream<Result<Frame<Bytes>, Infallible>>> =
            StreamBody::new(tokio_stream::wrappers::UnboundedReceiverStream::new(
                response_receiver,
            ));
        fn make_response(
            code: StatusCode,
        ) -> Result<
            Response<
                StreamBody<
                    tokio_stream::wrappers::UnboundedReceiverStream<
                        Result<Frame<Bytes>, Infallible>,
                    >,
                >,
            >,
            Infallible,
        > {
            let (_, mut rx) = mpsc::unbounded_channel::<Result<Frame<Bytes>, Infallible>>();
            rx.close();

            let response_stream =
                StreamBody::new(tokio_stream::wrappers::UnboundedReceiverStream::new(rx));
            Ok(Response::builder()
                .status(code)
                .body(response_stream)
                .unwrap())
        }

        if req.method() != Method::GET {
            let res = make_response(StatusCode::METHOD_NOT_ALLOWED);
            return Box::pin(async { res });
        }

        let uri = req.uri();

        let path_segments: Vec<&str> = uri.path().split('/').filter(|v| !v.is_empty()).collect();
        if path_segments.len() != 2 || !path_segments[1].ends_with(".flv") {
            return Box::pin(async { make_response(StatusCode::BAD_REQUEST) });
        }
        let app = path_segments[0];
        let stream_name = path_segments[1].strip_suffix(".flv").unwrap();

        let query_map: HashMap<String, String> = uri
            .query()
            .map(|v| {
                url::form_urlencoded::parse(v.as_bytes())
                    .into_owned()
                    .collect()
            })
            .unwrap_or_else(HashMap::new);

        let mut session = HttpFlvSession::new(
            HttpFlvSessionConfig {
                chunk_size: self.config.chunk_size,
                write_timeout_ms: self.config.write_timeout_ms,
                read_timeout_ms: self.config.read_timeout_ms,
            },
            self.stream_center_event_sender.clone(),
            StreamProperties {
                app: app.to_string(),
                stream_name: stream_name.to_string(),
                stream_type: Default::default(),
                stream_context: query_map,
            },
            response_sender,
        );

        tokio::spawn(async move {
            let _ = session.serve_pull_request().await;
            let _ = session.unsubscribe_from_stream_center().await;
        });

        let response: Response<
            StreamBody<
                tokio_stream::wrappers::UnboundedReceiverStream<Result<Frame<Bytes>, Infallible>>,
            >,
        > = Response::builder()
            .header(header::CONTENT_TYPE, "video/x-flv")
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
            .header(header::TRANSFER_ENCODING, "chunked")
            .status(StatusCode::OK)
            .body(response_stream)
            .unwrap();
        Box::pin(async { Ok(response) })
    }
}
