use std::{collections::HashMap, convert::Infallible, pin::Pin};

use http_body_util::{BodyExt, Empty, Full, StreamBody};
use hyper::{
    Method, Request, Response, StatusCode,
    body::{Body, Bytes, Frame, Incoming},
    header,
    server::conn::http1,
    service::Service,
};
use hyper_util::rt::{TokioIo, TokioTimer};
use stream_center::events::StreamCenterEvent;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio_stream::{StreamExt, wrappers::UnboundedReceiverStream};
use tokio_util::bytes::BytesMut;
use url::{Host, Url};

use crate::{
    config::{HttpFlvServerConfig, HttpFlvSessionConfig},
    errors::{HttpFlvServerError, HttpFlvServerResult},
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
        let mut host: Option<&str> = uri.host();
        if host.is_none() {
            host = req
                .headers()
                .get("host")
                .map(|v| v.to_str().unwrap_or("0.0.0.0"));
        }

        let uri = format!("http://{}{}", host.unwrap_or("0.0.0.0"), uri.path());
        let url = Url::parse(uri.as_str()).expect("Could I get a bad url from hyper?");

        let app;
        let mut stream_name;
        if let Some(path) = url.path_segments() {
            let path_params: Vec<&str> = path.collect();
            {
                if path_params.len() != 2 || !path_params[1].ends_with(".flv") {
                    let res = make_response(StatusCode::BAD_REQUEST);
                    return Box::pin(async { res });
                }
            }

            app = path_params.get(0).expect("this cannot be none").to_string();
            stream_name = path_params.get(1).expect("this cannot be none").to_string();

            stream_name = stream_name
                .strip_suffix(".flv")
                .expect("this cannot be none")
                .to_string();
        } else {
            let res = make_response(StatusCode::BAD_REQUEST);
            return Box::pin(async { res });
        }

        let mut query_map = HashMap::new();
        for (key, value) in url.query_pairs() {
            query_map.insert(key.to_string(), value.to_string());
        }

        let mut session = HttpFlvSession::new(
            HttpFlvSessionConfig {
                chunk_size: self.config.chunk_size,
                write_timeout_ms: self.config.write_timeout_ms,
                read_timeout_ms: self.config.read_timeout_ms,
            },
            self.stream_center_event_sender.clone(),
            StreamProperties {
                app,
                stream_name,
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