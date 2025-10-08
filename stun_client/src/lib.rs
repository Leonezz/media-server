use std::{
    net::{IpAddr, SocketAddr},
    pin::Pin,
};

use scopeguard::defer;
use stun_formats::{attribute::STUNAttribute, header::TransactionId, message::STUNMessage};
use stun_server::client::STUNClientResult;
use tokio::{select, task::block_in_place};
use unified_io::{UnifiedIO, tcp::TcpIO, udp::UdpIO};
use utils::net::protocol::Protocol;

use crate::config::AppConfig;

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
    let io = match config.protocol {
        Protocol::Tcp => {
            let socket = match config.local_addr {
                IpAddr::V4(_) => tokio::net::TcpSocket::new_v4(),
                IpAddr::V6(_) => tokio::net::TcpSocket::new_v6(),
            };
            if let Err(err) = socket {
                eprintln!("error construct tcp socket: {}", err);
                return;
            }
            let socket = socket.unwrap();
            if let Err(err) = socket.bind(local_addr) {
                eprintln!("error binding to {} , err: {}", local_addr, err);
                return;
            }
            let remote_addrs = tokio::net::lookup_host(config.server.clone()).await;
            if let Err(err) = remote_addrs {
                eprintln!("--server {} is not valid address: {}", config.server, err);
                return;
            }
            let remote_addrs: Vec<_> = remote_addrs.unwrap().collect();
            if remote_addrs.is_empty() {
                eprintln!("cannot resolve address from server: {}", config.server);
                return;
            }

            match socket.connect(remote_addrs[0]).await {
                Ok(tcp) => Box::pin(TcpIO::new(tcp)) as Pin<Box<dyn UnifiedIO>>,
                Err(err) => {
                    eprintln!("error connect to {}: {}", config.server, err);
                    return;
                }
            }
        }
        Protocol::Udp => {
            let udp = match tokio::net::UdpSocket::bind(local_addr).await {
                Ok(udp) => udp,
                Err(err) => {
                    eprintln!(
                        "error creating a udp socket with local addr: {}, err: {}",
                        local_addr, err
                    );
                    return;
                }
            };
            match udp.connect(config.server.clone()).await {
                Ok(()) => Box::pin(UdpIO::from_inner(udp)) as Pin<Box<dyn UnifiedIO>>,
                Err(err) => {
                    eprintln!("error connect to {}: {}", config.server, err);
                    return;
                }
            }
        }
    };

    let (result_tx, mut result_rx) = tokio::sync::mpsc::channel(10);
    let client = stun_server::client::STUNClient::new(io, local_addr, result_tx).await;
    defer! {
        block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let _ = client.close().await;
            });
        });
    }

    let transaction_id = TransactionId::new_random();
    let binding_request = STUNMessage::builder()
        .request()
        .binding()
        .transaction_id(transaction_id)
        .finger_print()
        .unwrap()
        .build();
    tracing::debug!("bing_request: {:?}", binding_request);
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
                    stun_formats::header::STUNMessageClass::SuccessResponse => {
                        tracing::debug!("got success response: {:?}", response);
                        if let Some(addr) =
                            response.get_attribute(stun_formats::attribute::AttrType::XorMappedAddress).or(
                                response.get_attribute(stun_formats::attribute::AttrType::MappedAddress)
                            )
                        {
                            tracing::debug!("mapped address: {:?}", addr);
                            let addr = match addr {
                                STUNAttribute::XorMappedAddress(addr) => {
                                    tracing::info!("addr: [{}]:{}", addr.address(), addr.port());
                                    Some((addr.address(), addr.port()))
                                }
                                  STUNAttribute::MappedAddress(addr) => {
                                    tracing::info!("addr: [{}]:{}", addr.address(), addr.port());
                                    Some((addr.address(), addr.port()))
                                  }
                                _ => {
                                    tracing::error!("unexpected attribute: {:?}", addr);
                                    None
                                }
                            };
                            if let Some((ip, port)) = addr {
                                println!("bind test success, mapped address: [{}]:{}", ip, port);
                            }
                        }
                    }
                    stun_formats::header::STUNMessageClass::ErrorResponse => {
                        tracing::debug!("got error response: {:?}", response);
                        if let Some(STUNAttribute::ErrorCode(error_code)) =
                            response.get_attribute(stun_formats::attribute::AttrType::ErrorCode)
                        {
                            eprintln!(
                                "error_code: {:?}, reason: {}",
                                error_code.error_code(),
                                error_code.reason_phrase()
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
