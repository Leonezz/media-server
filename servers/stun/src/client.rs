use connection::connection::Outgoing;
use futures::{FutureExt, select};
use std::net::SocketAddr;
use std::sync::Arc;
use stun_formats::message::Message;
use tokio::sync::{
    RwLock,
    mpsc::{Receiver, Sender, channel},
};

use crate::{
    agent::{Agent, AgentCommand, AgentEvent},
    errors::{StunSessionError, StunSessionResult},
};

#[derive(Debug, Clone)]
pub enum STUNClientResult {
    Response(Message),
    Error(String),
}

pub struct STUNClient {
    message_tx: tokio::sync::mpsc::Sender<Outgoing<Message>>,
    agent_command_tx: Arc<Sender<AgentCommand>>,
    close_tx: Sender<()>,
}

impl STUNClient {
    pub async fn new(
        message_tx: tokio::sync::mpsc::Sender<Outgoing<Message>>,
        message_rx: tokio::sync::mpsc::Receiver<Message>,
        local_addr: SocketAddr,
        result_tx: Sender<STUNClientResult>,
    ) -> Self {
        let (agent_command_tx, agent_command_rx) = channel(100);
        let (agent_event_tx, agent_event_rx) = channel(100);
        let (close_tx, close_rx) = channel(10);
        let result_tx = Arc::new(result_tx);

        tokio::spawn(Self::run_observe(
            message_tx.clone(),
            Arc::new(RwLock::new(agent_event_rx)),
            result_tx.clone(),
        ));

        let agent_command_tx = Arc::new(agent_command_tx);
        let agent = Agent::new(agent_command_rx, agent_event_tx);
        tokio::spawn(async {
            let _ = agent.run().await.inspect_err(|err| {
                tracing::error!("agent ended with error: {}", err);
            });
        });

        tokio::spawn(Self::run_read(
            close_rx,
            message_rx,
            local_addr,
            Arc::clone(&agent_command_tx),
            result_tx.clone(),
        ));
        Self {
            message_tx,
            agent_command_tx,
            close_tx,
        }
    }

    pub async fn send(&self, request: Message) -> StunSessionResult<()> {
        let (flush_tx, flush_rx) = tokio::sync::oneshot::channel();
        self.message_tx
            .send((request.clone(), Some(flush_tx)))
            .await
            .map_err(|err| {
                StunSessionError::ChannelError(format!("error sending message to socket: {}", err))
            })?;
        let _ = flush_rx.await.map_err(|err| {
            StunSessionError::ChannelError(format!("error waiting for flush ack: {}", err))
        })?;
        self.agent_command_tx
            .send(AgentCommand::SendRequest(request))
            .await
            .map_err(|err| StunSessionError::ChannelError(format!("{}", err)))?;
        Ok(())
    }

    pub async fn close(&self) -> StunSessionResult<()> {
        self.close_tx.send(()).await.map_err(|err| {
            StunSessionError::ChannelError(format!("error sending close single: {}", err))
        })?;
        Ok(())
    }

    async fn run_observe(
        message_tx: tokio::sync::mpsc::Sender<Outgoing<Message>>,
        agent_event_rx: Arc<RwLock<Receiver<AgentEvent>>>,
        result_tx: Arc<Sender<STUNClientResult>>,
    ) {
        loop {
            let res = agent_event_rx.write().await.recv().await;
            if let Some(res) = res {
                match res {
                    AgentEvent::Retransmit(message) => {
                        if let Err(err) = message_tx.send((message, None)).await {
                            let _ = result_tx
                                .send(STUNClientResult::Error(format!(
                                    "error sending retransmit message: {}",
                                    err
                                )))
                                .await
                                .inspect_err(|err| {
                                    tracing::error!("error send error event to observer: {}", err);
                                });
                        }
                    }
                    AgentEvent::OutgoingMessage(message) => {
                        tracing::info!("got message: {:?}", message);
                        let _ = result_tx
                            .send(STUNClientResult::Response(message))
                            .await
                            .inspect_err(|err| {
                                tracing::error!("error sending response to observer: {}", err);
                            });
                    }
                    AgentEvent::Timeout(message) => {
                        tracing::info!("message timeout: {:?}", message);
                        let _ = result_tx
                            .send(STUNClientResult::Error("timeout".to_string()))
                            .await
                            .inspect_err(|err| {
                                tracing::error!("error sending timeout to observer: {}", err);
                            });
                    }
                }
            } else {
                return;
            }
        }
    }

    async fn run_read(
        mut close_rx: Receiver<()>,
        mut message_rx: tokio::sync::mpsc::Receiver<Message>,
        local_addr: SocketAddr,
        agent_command_tx: Arc<Sender<AgentCommand>>,
        result_tx: Arc<Sender<STUNClientResult>>,
    ) {
        loop {
            select! {
                res = message_rx.recv().fuse() => {
                    let error = match res {
                        Some(message) => {
                            if let Err(err) = agent_command_tx
                                .send(AgentCommand::IncomingMessage((message.clone(), local_addr)))
                                .await
                            {
                                Some(STUNClientResult::Error(format!(
                                    "error processing message: {:?}, err: {}",
                                    message, err
                                )))
                            } else {
                                None
                            }
                        }
                        None => None,
                    };
                    if let Some(err) = error {
                        let _ = result_tx.send(err).await.inspect_err(|err| {
                            tracing::error!("error sending resut to observer: {}", err);
                        });
                    }
                }
                _ = close_rx.recv().fuse() => {
                    return;
                }
            }
        }
    }
}
