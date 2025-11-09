use crate::{
    errors::HttpServerResult,
    routes::{self},
};
use axum::routing::{get, post};
use std::net::IpAddr;
use stream_center::events::StreamCenterEvent;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct HttpServerContext {
    // ip address to serve on
    pub address: IpAddr,
    // port to serve on
    pub port: u16,
    // number of threads to use for executing futures
    pub workers: u64,
    pub stream_center_event_sender: mpsc::UnboundedSender<StreamCenterEvent>,
}

pub struct HttpServer {
    context: HttpServerContext,
}

impl HttpServer {
    pub fn new(
        address: IpAddr,
        port: u16,
        workers: u64,
        stream_center_event_sender: mpsc::UnboundedSender<StreamCenterEvent>,
    ) -> Self {
        Self {
            context: HttpServerContext {
                address,
                port,
                workers,
                stream_center_event_sender,
            },
        }
    }

    pub async fn run(&mut self) -> HttpServerResult<()> {
        tracing::info!(
            "http server is running at: tcp://{}:{}",
            self.context.address,
            self.context.port
        );
        let app = axum::Router::new()
            .route("/rest/v1/hello", get(routes::hello::hello))
            .route(
                "/live_stream/v1/{app}/{stream}",
                get(routes::httpflv::serve),
            )
            .with_state(self.context.clone())
            .route("/webrtc/v1/whip", post(routes::whip::serve));
        let listener = tokio::net::TcpListener::bind((self.context.address, self.context.port))
            .await
            .unwrap();
        match axum::serve(listener, app).await {
            Ok(res) => {
                tracing::info!("http server exit successfully");
                tracing::debug!("http server exit res: {:?}", res);
            }
            Err(err) => {
                tracing::error!("http server exit with err: {:?}", err);
            }
        }

        Ok(())
    }
}
