use crate::{
    connection::{Connection, Outgoing},
    errors::ConnResult,
    io::{ClientEndpointIo, ConnectionIo, ServerEndpointIo},
};
use std::net::SocketAddr;
use utils::traits;

#[derive(Debug)]
pub struct ClientEndpoint {
    io: ClientEndpointIo,
}

impl ClientEndpoint {
    pub fn new_tcp(local_addr: SocketAddr) -> ConnResult<Self> {
        let io = ClientEndpointIo::new_tcp(local_addr)?;
        Ok(Self { io })
    }

    pub fn new_udp(local_addr: SocketAddr) -> ConnResult<Self> {
        let io = ClientEndpointIo::new_udp(local_addr)?;
        Ok(Self { io })
    }

    pub async fn connect<M>(
        self,
        remote_addr: SocketAddr,
    ) -> ConnResult<(
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

    pub async fn accept<M>(
        &mut self,
    ) -> ConnResult<(
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
}
