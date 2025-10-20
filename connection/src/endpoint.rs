use crate::{
    connection::{Connection, Outgoing},
    io::{ClientEndpointIo, ConnectionIo, ServerEndpointIo},
};
use iana_formats::protocol_numbers::ProtocolNumberStatic;
use std::net::SocketAddr;
use utils::traits;
const SUPPORTED_PROTOCOLS: &[iana_formats::protocol_numbers::Protocol] = &[
    iana_formats::protocol_numbers::UDP::PROTOCOL,
    iana_formats::protocol_numbers::TCP::PROTOCOL,
];
#[derive(Debug)]
pub struct ClientEndpoint {
    io: ClientEndpointIo,
}

impl ClientEndpoint {
    pub fn new_tcp(local_addr: SocketAddr) -> std::io::Result<Self> {
        let io = ClientEndpointIo::new_tcp(local_addr)?;
        Ok(Self { io })
    }

    pub async fn new_udp(local_addr: SocketAddr) -> std::io::Result<Self> {
        let io = ClientEndpointIo::new_udp(local_addr).await?;
        Ok(Self { io })
    }

    pub async fn new(
        protocol: iana_formats::protocol_numbers::Protocol,
        local_addr: SocketAddr,
    ) -> std::io::Result<Self> {
        if !SUPPORTED_PROTOCOLS.contains(&protocol) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                format!("unsupported protocol: {:?}", protocol),
            ));
        }

        match protocol {
            iana_formats::protocol_numbers::UDP::PROTOCOL => Ok(Self::new_udp(local_addr).await?),
            iana_formats::protocol_numbers::TCP::PROTOCOL => Ok(Self::new_tcp(local_addr)?),
            _ => unreachable!("unsupported protocol"),
        }
    }

    pub async fn connect<M>(
        self,
        remote_addr: SocketAddr,
    ) -> std::io::Result<(
        Connection<ConnectionIo, M::Codec, M::Error, M::In, M::Out>,
        tokio::sync::mpsc::Sender<Outgoing<M::Out>>,
        tokio::sync::mpsc::Receiver<M::In>,
    )>
    where
        M: traits::protocol_message::ProtocolMessage,
    {
        let io = self.io.connect(remote_addr).await?;
        Ok(Connection::<ConnectionIo, M::Codec, M::Error, M::In, M::Out>::new(io, M::codec()))
    }

    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.io.local_addr()
    }
}

#[derive(Debug)]
pub struct ServerEndpoint {
    io: ServerEndpointIo,
}

impl ServerEndpoint {
    pub async fn new_tcp(local_addr: SocketAddr) -> std::io::Result<Self> {
        let io = ServerEndpointIo::new_tcp(local_addr).await?;
        Ok(Self { io })
    }

    pub async fn new_udp(local_addr: SocketAddr) -> std::io::Result<Self> {
        let io = ServerEndpointIo::new_udp(local_addr).await?;
        Ok(Self { io })
    }

    pub async fn new(
        protocol: iana_formats::protocol_numbers::Protocol,
        local_addr: SocketAddr,
    ) -> std::io::Result<Self> {
        if !SUPPORTED_PROTOCOLS.contains(&protocol) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                format!("unsupported protocol: {:?}", protocol),
            ));
        }

        match protocol {
            iana_formats::protocol_numbers::UDP::PROTOCOL => Self::new_udp(local_addr).await,
            iana_formats::protocol_numbers::TCP::PROTOCOL => Self::new_tcp(local_addr).await,
            _ => unreachable!("unsupported protocol"),
        }
    }

    pub async fn accept<M>(
        &mut self,
    ) -> std::io::Result<(
        SocketAddr,
        Connection<ConnectionIo, M::Codec, M::Error, M::In, M::Out>,
        tokio::sync::mpsc::Sender<Outgoing<M::Out>>,
        tokio::sync::mpsc::Receiver<M::In>,
    )>
    where
        M: traits::protocol_message::ProtocolMessage,
    {
        let io = self.io.accept().await?;
        let remote_addr = io.remote_addr();
        let (conn, tx, rx) =
            Connection::<ConnectionIo, M::Codec, M::Error, M::In, M::Out>::new(io, M::codec());
        Ok((remote_addr, conn, tx, rx))
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.io.local_addr()
    }
}
