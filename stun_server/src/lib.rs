pub mod config;
pub mod errors;

use config::AppConfig;
use tokio::select;
pub async fn app_run<F>(config: AppConfig, stop: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    let server = stun_server::server::STUNServer::new(
        config.protocol,
        std::net::SocketAddr::new(config.local_addr, config.local_port),
    );
    select! {
        res = server.run() => {
            if let Err(err) = res {
                tracing::error!("stun server ended with err: {}", err);
                return;
            }
        },
        _ = stop => {
            tracing::info!("stun server gracefully exited");
            return;
        }
    }
}
