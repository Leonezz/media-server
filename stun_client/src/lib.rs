use crate::config::AppConfig;
use scopeguard::defer;
use std::net::{SocketAddr, ToSocketAddrs};
use stun_formats::{
    attributes::rfc8489::{ErrorCodeAttribute, MappedAddressAttribute, XorMappedAddressAttribute},
    header::TransactionId,
    message::Message,
};
use stun_server::client::STUNClientResult;
use tokio::{select, task::block_in_place};
use utils::net::protocol::Protocol;

pub mod config;
pub mod errors;

pub async fn app_run<F>(config: AppConfig, stop: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    println!(
        "start with local address: [{}]:{}",
        config.local_addr, config.local_port
    );
    let local_addr = SocketAddr::new(config.local_addr, config.local_port);
    let remote_addrs = config
        .server
        .to_socket_addrs()
        .inspect_err(|err| {
            eprintln!("error resolving server address: {}", err);
            std::process::exit(1);
        })
        .unwrap();
    let mut client = None;
    for remote in remote_addrs {
        tracing::debug!("trying server address: {}", remote);
        let endpoint = match config.protocol {
            Protocol::Tcp => connection::endpoint::ClientEndpoint::new_tcp(local_addr),
            Protocol::Udp => connection::endpoint::ClientEndpoint::new_udp(local_addr).await,
        }
        .inspect_err(|err| {
            eprintln!("error creating socket: {}", err);
            std::process::exit(1);
        })
        .unwrap();

        match endpoint.connect::<Message>(remote).await {
            Ok((conn, message_tx, message_rx)) => {
                tracing::info!("connected to server: {}", remote);
                let (result_tx, result_rx) = tokio::sync::mpsc::channel(10);
                tokio::spawn(async move {
                    let _ = conn.await.inspect_err(|err| {
                        tracing::error!("connection to server {} closed with err: {}", remote, err);
                    });
                });
                let stun_client = stun_server::client::STUNClient::new(
                    message_tx, message_rx, local_addr, result_tx,
                )
                .await;
                client = Some((stun_client, result_rx));
                break;
            }
            Err(err) => {
                tracing::error!(
                    "error connecting to server {}: {}, trying next",
                    remote,
                    err
                );
            }
        }
    }

    if client.is_none() {
        eprintln!("failed to connect to any server address");
        return;
    }
    let (client, mut result_rx) = client.unwrap();
    defer! {
        block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let _ = client.close().await;
            });
        });
    }

    let transaction_id = TransactionId::new_random();
    let binding_request = Message::builder()
        .request()
        .binding()
        .transaction_id(transaction_id)
        .finger_print()
        .unwrap()
        .build();
    tracing::debug!("bind_request: {:?}", binding_request);
    if let Err(err) = binding_request {
        eprintln!("error building request: {}", err);
        return;
    }
    if let Err(err) = client.send(binding_request.unwrap()).await {
        eprintln!("error sending request: {}", err);
        return;
    };

    select! {
        res = result_rx.recv() => match res {
            Some(result) => match result {
                STUNClientResult::Error(err) => {
                    tracing::error!("got error result: {}", err);
                }
                STUNClientResult::Response(response) if response.transaction_id().eq(&transaction_id) => match response.message_class() {
                    stun_formats::header::MessageClass::SuccessResponse => {
                        tracing::debug!("got success response: {:?}", response);
                        if let Some(addr) =
                            response.get_attribute_ext::<XorMappedAddressAttribute>().map(|item| (item.ip(), item.port())).or(
                                response.get_attribute_ext::<MappedAddressAttribute>().map(|item| (item.address(), item.port()))
                            )
                        {
                            tracing::debug!("mapped address: {:?}", addr);
                            let (ip, port) = addr;
                            println!("bind test success, mapped address: [{}]:{}", ip, port);
                        } else {
                            eprintln!("bind resposne success but no address found");
                        }
                    }
                    stun_formats::header::MessageClass::ErrorResponse => {
                        tracing::debug!("got error response: {:?}", response);
                        if let Some(error_code) =
                            response.get_attribute_ext::<ErrorCodeAttribute>()
                        {
                            eprintln!(
                                "error_code: {:?}, reason: {}",
                                error_code.error_code(),
                                error_code.reason()
                            );
                        }
                    }
                    _ => {
                        eprintln!("got unexpected response: {:?}", response)
                    }
                }
                STUNClientResult::Response(response) => {
                    eprintln!("got response with unexpected transaction_id: {:?}", response);
                }
            }
            None => {
                tracing::debug!("result channel has been closed");
            }
        },

        _ = stop => {}
    }
}
