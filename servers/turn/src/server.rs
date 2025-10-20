use crate::{errors::TurnSessionResult, session::Session};
use iana_formats::protocol_numbers::ProtocolNumberStatic;
use std::net::{SocketAddr, SocketAddrV4, SocketAddrV6};
use turn_formats::message::Message;

pub struct TurnServer {
    protocol: iana_formats::protocol_numbers::Protocol,
    address: SocketAddr,
    ipv4_address: SocketAddrV4,
    ipv6_address: SocketAddrV6,
}

impl TurnServer {
    pub fn new(
        protocol: iana_formats::protocol_numbers::Protocol,
        address: SocketAddr,
        ipv4_address: SocketAddrV4,
        ipv6_address: SocketAddrV6,
    ) -> TurnSessionResult<Self> {
        if ![
            iana_formats::protocol_numbers::UDP::PROTOCOL,
            iana_formats::protocol_numbers::TCP::PROTOCOL,
        ]
        .contains(&protocol)
        {
            return Err(crate::errors::TurnSessionError::UnsupportedProtocol(
                protocol,
            ));
        }
        Ok(Self {
            protocol,
            address,
            ipv4_address,
            ipv6_address,
        })
    }

    pub async fn run(self) -> TurnSessionResult<()> {
        tracing::info!(
            "turn server is starting at: {}://{}",
            iana_formats::protocol_numbers::from_protocol(self.protocol)
                .map(|item| item.keyword())
                .unwrap_or("unknown"),
            self.address
        );

        let mut endpoint =
            connection::endpoint::ServerEndpoint::new(self.protocol, self.address).await?;
        while let Ok((remote_addr, conn, message_tx, message_rx)) =
            endpoint.accept::<Message>().await
        {
            tracing::info!("got new connection from {}", remote_addr);
            tokio::spawn(async move {
                let _ = conn.await.inspect_err(|err| {
                    tracing::error!("connection from {} closed with err: {}", remote_addr, err);
                });
                tracing::info!(
                    "connection closed for {} <--> {}",
                    remote_addr,
                    self.address
                );
            });
            let session = Session::new(
                self.protocol,
                self.address,
                remote_addr,
                self.ipv4_address,
                self.ipv6_address,
                message_tx,
                message_rx,
            );
            tokio::spawn(async move {
                let _ = session.run().await.inspect_err(|err| {
                    tracing::warn!(
                        "turn session closed for remote: {}, local: {} with err: {}",
                        remote_addr,
                        self.address,
                        err
                    );
                });
                tracing::info!(
                    "turn session detached for remote: {}, local: {}",
                    remote_addr,
                    self.address
                );
            });
        }
        Ok(())
    }
}
