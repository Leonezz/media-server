use std::{io, net::SocketAddr, sync::Arc};

use crate::{
    agent::{Agent, AgentCommand, AgentEvent},
    errors::STUNSessionResult,
};
use futures::{SinkExt, StreamExt};
use stun_formats::message::{STUNMessage, STUNMessageFramed};
use unified_io::{UnifiyStreamed, tcp::TcpIO};
use utils::{
    net::protocol::Protocol,
    traits::{dynamic_sized_packet::DynamicSizedPacket, reader::ReadFrom, writer::WriteTo},
};

pub struct STUNServer {
    protocol: Protocol,
    address: SocketAddr,
}

impl STUNServer {
    pub fn new(protocol: Protocol, address: SocketAddr) -> Self {
        Self { protocol, address }
    }

    pub async fn run(self) -> STUNSessionResult<()> {
        tracing::info!(
            "stun server is starting at: {}://{}",
            self.protocol,
            self.address
        );

        let (agent_command_tx, agent_command_rx) = tokio::sync::mpsc::channel(100);
        let (agent_event_tx, mut agent_event_rx) = tokio::sync::mpsc::channel(100);
        let (agent_event_broadcaster, _) = tokio::sync::broadcast::channel(1000);
        let agent = Agent::new(agent_command_rx, agent_event_tx);
        tokio::spawn(async move {
            if let Err(err) = agent.run().await {
                tracing::error!("agent exit with err: {}", err);
            }
        });
        let agent_event_broadcaster_copy = agent_event_broadcaster.clone();
        tokio::spawn(async move {
            loop {
                match agent_event_rx.recv().await {
                    Some(event) => match agent_event_broadcaster_copy.send(event.clone()) {
                        Ok(size) => {
                            tracing::debug!(
                                "broadcasting agent event success, current {} subscribers",
                                size
                            );
                        }
                        Err(err) => {
                            tracing::error!(
                                "error broadcasting agent event: {:?}, err: {}",
                                event,
                                err
                            );
                        }
                    },
                    None => {
                        tracing::info!("agent event tx closed, exiting");
                    }
                }
            }
        });

        match self.protocol {
            Protocol::Tcp => {
                return Self::run_tcp(
                    self.address,
                    agent_command_tx.clone(),
                    agent_event_broadcaster.subscribe(),
                )
                .await;
            }
            Protocol::Udp => {
                let socket = tokio::net::UdpSocket::bind(self.address).await?;
                return Self::run_udp(
                    socket,
                    agent_command_tx.clone(),
                    agent_event_broadcaster.subscribe(),
                )
                .await;
            }
        }
    }

    async fn run_udp(
        socket: tokio::net::UdpSocket,
        agent_command_tx: tokio::sync::mpsc::Sender<AgentCommand>,
        agent_event_rx: tokio::sync::broadcast::Receiver<AgentEvent>,
    ) -> STUNSessionResult<()> {
        let (bytes_tx, mut bytes_rx) = tokio::sync::mpsc::channel::<(Vec<u8>, SocketAddr)>(1_000);
        let socket = Arc::new(socket);
        let send = socket.clone();
        tokio::spawn(async move {
            while let Some((bytes, addr)) = bytes_rx.recv().await {
                if let Err(err) = send.send_to(&bytes, &addr).await {
                    tracing::error!("error sending bytes to {}, err: {}", addr, err);
                }
            }
        });
        loop {
            let mut read_buffer = Vec::with_capacity(1024);
            let (len, remote) = socket.recv_buf_from(&mut read_buffer).await?;
            let mut reader = io::Cursor::new(read_buffer);
            let message = STUNMessage::read_from(&mut reader);
            if let Err(err) = message {
                tracing::error!(
                    "got invalid message from {} bytes datagram, ignore, remote addr: {}, err: {}",
                    len,
                    remote,
                    err
                );
                continue;
            }
            let message = message.unwrap();
            tracing::info!("got stun message: {:?}", message);
            let command_tx = agent_command_tx.clone();
            if !matches!(
                message.message_class(),
                stun_formats::header::STUNMessageClass::Request
            ) {
                tracing::warn!(
                    "got a stun message that is not a request: {:?}, ignore",
                    message
                );
                continue;
            }
            let mut agent_event_rx_copy = agent_event_rx.resubscribe();
            let bytes_tx_copy = bytes_tx.clone();
            tokio::spawn(async move {
                if let Err(err) = command_tx
                    .send(crate::agent::AgentCommand::IncomingMessage((
                        message.clone(),
                        remote,
                    )))
                    .await
                {
                    tracing::error!(
                        "error sending new incoming message to agent, message: {:?}, remote: {}, err: {}",
                        message,
                        remote,
                        err
                    );
                    return;
                }

                loop {
                    match agent_event_rx_copy.recv().await {
                        Ok(event) => match event {
                            AgentEvent::OutgoingMessage(response)
                                if response.transaction_id().eq(message.transaction_id()) =>
                            {
                                tracing::info!("got response from agent: {:?}", response);
                                let mut bytes =
                                    Vec::with_capacity(response.get_packet_bytes_count());
                                if let Err(err) = response.write_to(&mut bytes) {
                                    tracing::error!(
                                        "error serialize stun message to bytes, message: {:?}, err: {}",
                                        response,
                                        err
                                    );
                                    return;
                                }
                                if let Err(err) = bytes_tx_copy.send((bytes, remote)).await {
                                    tracing::error!(
                                        "error sending stun message bytes to remote: {}, err: {}",
                                        remote,
                                        err
                                    );
                                    return;
                                }
                            }
                            _ => {}
                        },
                        Err(err) => {
                            tracing::error!(
                                "error receiving response from agent for request: {:?}, err: {}",
                                message,
                                err
                            );
                            return;
                        }
                    }
                }
            });
        }
    }

    async fn run_tcp(
        socket_addr: SocketAddr,
        agent_command_tx: tokio::sync::mpsc::Sender<AgentCommand>,
        agent_event_rx: tokio::sync::broadcast::Receiver<AgentEvent>,
    ) -> STUNSessionResult<()> {
        let listener = tokio::net::TcpListener::bind(socket_addr).await?;
        loop {
            let (stream, remote_addr) = listener.accept().await?;
            let mut codec = UnifiyStreamed::new(Box::pin(TcpIO::new(stream)), STUNMessageFramed {});
            let command_tx = agent_command_tx.clone();
            let mut agent_event_rx_copy = agent_event_rx.resubscribe();

            tokio::spawn(async move {
                loop {
                    while let Some(message) = codec.next().await {
                        if let Err(err) = message {
                            tracing::error!(
                                "trying to read stun message from remote {} failed, err: {}",
                                remote_addr,
                                err
                            );
                            return;
                        }
                        let message = message.unwrap();
                        tracing::info!("got stun message: {:?}", message);
                        if !matches!(
                            message.message_class(),
                            stun_formats::header::STUNMessageClass::Request
                        ) {
                            tracing::warn!(
                                "got a stun message that is not a request: {:?}, ignore",
                                message
                            );
                            continue;
                        }

                        if let Err(err) = command_tx
                            .send(crate::agent::AgentCommand::IncomingMessage((
                                message.clone(),
                                remote_addr,
                            )))
                            .await
                        {
                            tracing::error!(
                                "error sending new incoming message to agent, message: {:?}, remote: {}, err: {}",
                                message,
                                remote_addr,
                                err
                            );
                            return;
                        }

                        loop {
                            match agent_event_rx_copy.recv().await {
                                Ok(event) => match event {
                                    AgentEvent::OutgoingMessage(response)
                                        if response
                                            .transaction_id()
                                            .eq(message.transaction_id()) =>
                                    {
                                        tracing::info!("got response from agent: {:?}", response);
                                        if let Err(err) = codec.send(response).await {
                                            tracing::error!(
                                                "error sending stun message bytes to remote: {}, err: {}",
                                                remote_addr,
                                                err
                                            );
                                            return;
                                        }
                                    }
                                    _ => {}
                                },
                                Err(err) => {
                                    tracing::error!(
                                        "error receiving response from agent for request: {:?}, err: {}",
                                        message,
                                        err
                                    );
                                    return;
                                }
                            }
                        }
                    }
                }
            });
        }
    }
}
