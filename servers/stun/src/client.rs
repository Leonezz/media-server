use futures::{FutureExt, Sink, SinkExt, Stream, StreamExt, select};
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use stun_formats::errors::STUNMessageError;
use stun_formats::message::{STUNMessage, STUNMessageFramed};
use tokio::sync::Mutex;
use tokio::sync::{
    RwLock,
    mpsc::{Receiver, Sender, channel},
};
use unified_io::{UnifiedIO, UnifiyStreamed};

use crate::{
    agent::{Agent, AgentCommand, AgentEvent},
    errors::{STUNSessionError, STUNSessionResult},
};

#[derive(Debug, Clone)]
pub enum STUNClientResult {
    Response(STUNMessage),
    Error(String),
}

type MessageSource =
    Arc<Mutex<Pin<Box<dyn Sink<STUNMessage, Error = STUNMessageError> + Send + Sync>>>>;
type MessageSink =
    Arc<Mutex<Pin<Box<dyn Sink<STUNMessage, Error = STUNMessageError> + Send + Sync>>>>;

pub struct STUNClient {
    io: MessageSink,
    agent_command_tx: Arc<Sender<AgentCommand>>,
    close_tx: Sender<()>,
}

impl STUNClient {
    pub async fn new(
        io: Pin<Box<dyn UnifiedIO>>,
        local_addr: SocketAddr,
        result_tx: Sender<STUNClientResult>,
    ) -> Self {
        let (agent_command_tx, agent_command_rx) = channel(100);
        let (agent_event_tx, agent_event_rx) = channel(100);
        let (close_tx, close_rx) = channel(10);
        let streamd = UnifiyStreamed::new(io, STUNMessageFramed {});
        let (sink, stream) = streamd.split();
        let result_tx = Arc::new(result_tx);
        let sink: MessageSink = Arc::new(Mutex::new(Box::pin(sink)
            as Pin<Box<dyn Sink<STUNMessage, Error = STUNMessageError> + Send + Sync>>));
        tokio::spawn(Self::run_observe(
            sink.clone(),
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
            Box::pin(stream),
            local_addr,
            Arc::clone(&agent_command_tx),
            result_tx.clone(),
        ));
        Self {
            io: sink,
            agent_command_tx,
            close_tx,
        }
    }

    pub async fn send(&self, request: STUNMessage) -> STUNSessionResult<()> {
        self.io.lock().await.send(request.clone()).await?;
        self.io.lock().await.flush().await?;
        self.agent_command_tx
            .send(AgentCommand::SendRequest(request))
            .await
            .map_err(|err| STUNSessionError::ChannelError(format!("{}", err)))?;
        Ok(())
    }

    pub async fn close(&self) -> STUNSessionResult<()> {
        self.close_tx.send(()).await.map_err(|err| {
            STUNSessionError::ChannelError(format!("error sending close single: {}", err))
        })?;
        Ok(())
    }

    async fn run_observe(
        io: MessageSource,
        agent_event_rx: Arc<RwLock<Receiver<AgentEvent>>>,
        result_tx: Arc<Sender<STUNClientResult>>,
    ) {
        loop {
            let res = agent_event_rx.write().await.recv().await;
            if let Some(res) = res {
                match res {
                    AgentEvent::Retransmit(message) => {
                        if let Err(err) = io.lock().await.send(message).await {
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
        mut io: Pin<Box<dyn Stream<Item = Result<STUNMessage, STUNMessageError>> + Send + Sync>>,
        local_addr: SocketAddr,
        agent_command_tx: Arc<Sender<AgentCommand>>,
        result_tx: Arc<Sender<STUNClientResult>>,
    ) {
        loop {
            select! {
                res = io.next().fuse() => {
                    let error = match res {
                        Some(message) => {
                            match message {
                                Ok(message) => {
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
                                Err(err) => Some(STUNClientResult::Error(format!(
                                    "error reading message: {:?}",
                                    err
                                ))),
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
