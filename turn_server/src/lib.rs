use iana_formats::protocol_numbers::ProtocolNumberStatic;
use tokio::select;

pub async fn app_run<F>(stop: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    let local_ipv4_addr: std::net::SocketAddrV4 = "127.0.0.1:5799".parse().unwrap();
    let local_ipv6_addr: std::net::SocketAddrV6 = "[::1]:5799".parse().unwrap();
    let server = turn_server::server::TurnServer::new(
        iana_formats::protocol_numbers::UDP::PROTOCOL,
        local_ipv4_addr.into(),
        local_ipv4_addr,
        local_ipv6_addr,
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
