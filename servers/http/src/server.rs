use figment::{Figment, providers::Serialized};
use rocket::{Config, config::Ident, routes};
use stream_center::events::StreamCenterEvent;
use tokio::sync::mpsc;

use crate::{
    config::HttpServerConfig,
    errors::HttpServerResult,
    routes::{self, hello::hello},
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
        log::info!("http server is running, config: {:?}", self.context.config);
        let figment = Figment::from(Config {
            log_level: rocket::config::LogLevel::Off,
            ident: Ident::try_new("zhuwenq").unwrap(),
            ip_header: Some("X-Real-IP".into()),
            keep_alive: 5,
            ..Default::default()
        })
        .merge(Serialized::defaults(&self.context.config));

        match rocket::custom(figment)
            .manage(self.context.clone())
            .mount("/rest/v1", routes![hello])
            .mount("/live_stream/v1", routes![routes::httpflv::serve])
            .launch()
            .await
        {
            Ok(res) => {
                log::info!(
                    "http server exit successfully, config: {:?}",
                    self.context.config
                );
                log::debug!("http server exit res: {:?}", res);
            }
            Err(err) => {
                log::error!("http server exit with err: {:?}", err);
            }
        }

        Ok(())
    }
}
