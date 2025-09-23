use axum::routing::{get, post};
use stream_center::events::StreamCenterEvent;
use tokio::sync::mpsc;

use crate::{
    config::HttpServerConfig,
    errors::HttpServerResult,
    routes::{self},
};

#[derive(Clone)]
pub struct HttpServerContext {
    pub config: HttpServerConfig,
    pub stream_center_event_sender: mpsc::UnboundedSender<StreamCenterEvent>,
}

pub struct HttpServer {
    context: HttpServerContext,
}

impl HttpServer {
    pub fn new(
        config: HttpServerConfig,
        stream_center_event_sender: mpsc::UnboundedSender<StreamCenterEvent>,
    ) -> Self {
        Self {
            context: HttpServerContext {
                config,
                stream_center_event_sender,
            },
        }
    }

    pub async fn run(&mut self) -> HttpServerResult<()> {
        tracing::info!("http server is running, config: {:?}", self.context.config);
        let app = axum::Router::new()
            .route("/rest/v1/hello", get(routes::hello::hello))
            .route(
                "/live_stream/v1/{app}/{stream}",
                get(routes::httpflv::serve),
            )
            .with_state(self.context.clone())
            .route("/webrtc/v1/whip", post(routes::whip::serve));
        let listener =
            tokio::net::TcpListener::bind((self.context.config.address, self.context.config.port))
                .await
                .unwrap();
        match axum::serve(listener, app).await {
            Ok(res) => {
                tracing::info!(
                    "http server exit successfully, config: {:?}",
                    self.context.config
                );
                tracing::debug!("http server exit res: {:?}", res);
            }
            Err(err) => {
                tracing::error!("http server exit with err: {:?}", err);
            }
        }

        Ok(())
    }
}
