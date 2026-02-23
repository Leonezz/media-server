use crate::{
    agent::{Agent, AgentCommand, AgentEvent},
    errors::StunSessionResult,
};
use connection::connection::Outgoing;
use std::{io, net::SocketAddr};
use stun_formats::message::Message;
use utils::net::protocol::Protocol;

pub struct STUNServer {
    protocol: Protocol,
    address: SocketAddr,
}

impl STUNServer {
    pub fn new(protocol: Protocol, address: SocketAddr) -> Self {
        Self { protocol, address }
    }

    pub async fn run(self) -> StunSessionResult<()> {
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

        let mut endpoint = match self.protocol {
            Protocol::Tcp => connection::endpoint::ServerEndpoint::new_tcp(self.address).await?,
            Protocol::Udp => connection::endpoint::ServerEndpoint::new_udp(self.address).await?,
        };

        while let Ok((remote_addr, conn, message_tx, message_rx)) =
            endpoint.accept::<Message>().await
        {
            tracing::info!("got new connection from {}", remote_addr);
            tokio::spawn(async move {
                let _ = conn.await.inspect_err(|err| {
                    tracing::error!("connection from {} closed with err: {}", remote_addr, err);
                });
            });
            let agent_command_tx = agent_command_tx.clone();
            let agent_event_rx = agent_event_broadcaster.subscribe();
            tokio::spawn(async move {
                let _ = Self::handle_connection(
                    remote_addr,
                    message_tx,
                    message_rx,
                    agent_command_tx,
                    agent_event_rx,
                )
                .await
                .inspect_err(|err| {
                    tracing::error!(
                        "error handling connection from {}, err: {}",
                        remote_addr,
                        err
                    );
                });
            });
        }

        Ok(())
    }

    async fn handle_connection(
        remote_addr: SocketAddr,
        message_tx: tokio::sync::mpsc::Sender<Outgoing<Message>>,
        mut message_rx: tokio::sync::mpsc::Receiver<Message>,
        agent_command_tx: tokio::sync::mpsc::Sender<AgentCommand>,
        mut agent_event_rx: tokio::sync::broadcast::Receiver<AgentEvent>,
    ) -> StunSessionResult<()> {
        while let Some(message) = message_rx.recv().await {
            tracing::info!("got stun message: {:?}", message);
            if !matches!(
                message.message_class(),
                stun_formats::header::MessageClass::Request
            ) {
                tracing::warn!(
                    "got a stun message that is not a request: {:?}, ignore",
                    message
                );
                continue;
            }

            if let Err(err) = agent_command_tx
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
                return Err(std::io::Error::new(io::ErrorKind::BrokenPipe, err).into());
            }

            loop {
                match agent_event_rx.recv().await {
                    Ok(event) => match event {
                        AgentEvent::OutgoingMessage(response)
                            if response.transaction_id().eq(message.transaction_id()) =>
                        {
                            tracing::info!("got response from agent: {:?}", response);
                            let (flush_tx, flush_rx) = tokio::sync::oneshot::channel();
                            if let Err(err) = message_tx.send((response, Some(flush_tx))).await {
                                tracing::error!(
                                    "error sending stun message bytes to remote: {}, err: {}",
                                    remote_addr,
                                    err
                                );
                                return Err(
                                    std::io::Error::new(io::ErrorKind::BrokenPipe, err).into()
                                );
                            }
                            tracing::debug!("waiting for flush ack from connection");
                            let _ = flush_rx.await;
                            tracing::debug!("flush ack received, finish");
                            return Ok(());
                        }
                        _ => {}
                    },
                    Err(err) => {
                        tracing::error!(
                            "error receiving response from agent for request: {:?}, err: {}",
                            message,
                            err
                        );
                        return Err(std::io::Error::new(io::ErrorKind::BrokenPipe, err).into());
                    }
                }
            }
        }
        Ok(())
    }
}
