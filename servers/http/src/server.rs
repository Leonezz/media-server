use figment::{Figment, providers::Serialized};
use rocket::{Config, config::Ident, routes};
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
        let figment = Figment::from(Config {
            log_level: rocket::config::LogLevel::Off,
            ident: Ident::try_new("zhuwenq").unwrap(),
            ip_header: Some("X-Real-IP".into()),
            keep_alive: 5,
            ..Default::default()
        })
        .merge(Serialized::defaults(&self.context.config));

        let res = rocket::custom(figment)
            .manage(self.context.clone())
            .mount("/", routes![routes::httpflv::serve])
            .launch()
            .await;
        tracing::info!("{:?}", res);
        Ok(())
    }
}
