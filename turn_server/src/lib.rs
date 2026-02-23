pub mod cli;
use tokio::select;

pub async fn app_run<F>(stop: F, config: cli::TurnServerCli)
where
    F: Future<Output = ()> + Send + 'static,
{
    let local_addr = std::net::SocketAddr::new(config.localaddr, config.localport);
    let server = turn_server::server::TurnServer::new(
        config.protocol,
        local_addr,
        config.local_ipv4_addr,
        if config.use_ipv6 {
            Some(config.local_ipv6_addr)
        } else {
            None
        },
        config.use_icmp,
    )
    .unwrap();

    select! {
        res = server.run() => {
            if let Err(err) = res {
                tracing::error!("turn server ended with err: {}", err);
                return;
            }
        }
        _ = stop => {
            tracing::info!("turn server gracefully exited");
            return;
        }
    }
}
