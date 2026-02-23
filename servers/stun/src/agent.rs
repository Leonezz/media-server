use std::{
    collections::HashMap,
    net::SocketAddr,
    time::{Duration, Instant},
};

use rootcause::report;
use stun_formats::{
    attributes::rfc8489::XorMappedAddressAttribute, errors::StunMessageError,
    header::TransactionId, message::Message, methods::MethodExtStatic,
};
use tokio::sync::mpsc::{Receiver, Sender};
use utils::errors::context::{ContextExt, LocationExt};

use crate::errors::{StunSessionError, StunSessionResult};

#[derive(Debug)]
pub struct Transaction {
    request: Message,
    deadline: Instant,
    retransmit_times: u32,
    transation_start: Instant,
}

/// AgentCommand goes from App -> Agent
#[derive(Debug, Clone)]
pub enum AgentCommand {
    // any message a client or server received from socket,
    // a client received a resposne and emit it to agent through this,
    // a server received a request and emit it to agent through this,
    IncomingMessage((Message, SocketAddr)),
    SendRequest(Message), // MUST be a request
}

/// AgentEvent goes from Agent -> App
#[derive(Debug, Clone)]
pub enum AgentEvent {
    // any message a client or server received from socket,
    // a client will expect a response from this,
    // a server will expect a request from this,
    OutgoingMessage(Message),
    FurtherProcess((Message, SocketAddr)), // a message not understood by stun agent, further process needed
    Retransmit(Message),                   // MUST be a request
    Timeout(Message),                      // a request has timed out, MUST be a request
}

#[derive(Debug)]
pub struct Agent {
    transactions: HashMap<TransactionId, Transaction>,
    command_rx: Receiver<AgentCommand>,
    event_tx: Sender<AgentEvent>,
    rto: Duration,
    timer_interval: Duration,
    max_retransmit_times: u32,
    max_transaction_duration: Duration,
}

impl Agent {
    pub fn new(command_rx: Receiver<AgentCommand>, event_tx: Sender<AgentEvent>) -> Self {
        Self {
            transactions: Default::default(),
            command_rx,
            event_tx,
            rto: Duration::from_millis(500),
            timer_interval: Duration::from_millis(100),
            max_retransmit_times: 7,
            max_transaction_duration: Duration::from_millis(500).checked_mul(16).unwrap(),
        }
    }

    pub fn with_rto(mut self, rto: Duration) -> Self {
        self.rto = rto;
        self
    }

    pub fn with_timer_interval(mut self, timer_interval: Duration) -> Self {
        self.timer_interval = timer_interval;
        self
    }

    pub fn with_max_retransmit_times(mut self, max_retransmit_times: u32) -> Self {
        self.max_retransmit_times = max_retransmit_times;
        self
    }

    pub fn with_max_transaction_rto(mut self, max_transaction_duration: Duration) -> Self {
        self.max_transaction_duration = max_transaction_duration;
        self
    }

    fn on_timer(&mut self) -> StunSessionResult<Vec<AgentEvent>> {
        let now = Instant::now();
        let mut events = Vec::new();
        self.transactions.retain(|_, transaction| {
            if transaction.deadline > now {
                return true;
            }
            if transaction.retransmit_times >= self.max_retransmit_times
                || now - transaction.transation_start >= self.max_transaction_duration
            {
                events.push(AgentEvent::Timeout(transaction.request.clone()));
                return false;
            }
            transaction.retransmit_times += 1;
            transaction.deadline =
                now + self.rto.checked_mul(transaction.retransmit_times).unwrap();
            tracing::info!(
                "retransmit: {:?}, retransmit_times: {}, next_deadline in: {} ms",
                transaction.request,
                transaction.retransmit_times,
                (transaction.deadline - now).as_millis()
            );
            events.push(AgentEvent::Retransmit(transaction.request.clone()));
            true
        });
        Ok(events)
    }

    fn on_incoming_message(
        &mut self,
        message: (Message, SocketAddr),
    ) -> StunSessionResult<AgentEvent> {
        let (message, remote) = message;
        let transaction_id = message.transaction_id().clone();
        tracing::debug!(
            "incoming message: {} from {}, transaction_id: {}",
            message,
            remote,
            transaction_id
        );
        match message.check()
            .operation("checking incoming STUN message")
            .resource("remote", remote)
            .resource("transaction", transaction_id)
            .trace()
        {
            Ok(()) => {}
            Err(report) => {
                tracing::warn!(
                    "message from remote: {}, transaction_id: {}, check failed with error: {}",
                    remote,
                    transaction_id,
                    report,
                );
                if let Some(error) = report.downcast_current_context::<StunMessageError>() {
                    let mut response_builder = Message::builder()
                        .transaction_id(transaction_id)
                        .method(message.message_method().clone());
                    let prepare_error_response_res =
                        error.try_prepare_error_response(&mut response_builder);
                    if let Err(err) = prepare_error_response_res {
                        tracing::error!(
                            "trying to prepare error response from message checking error failed: {}, remote: {}, transaction_id: {}",
                            err,
                            remote,
                            transaction_id
                        );
                        return Err(err);
                    }
                    if let Ok(false) = prepare_error_response_res {
                        tracing::warn!("unable to convert error into error response, ignore");
                        return Err(report);
                    }
                    let response = response_builder
                        .finger_print()
                        .trace()?
                        .build()
                        .operation("building error response")
                        .resource("transaction", transaction_id)
                        .trace()?;
                    return Ok(AgentEvent::OutgoingMessage(response));
                } else {
                    return Err(report);
                }
            }
        };
        match message.message_class() {
            stun_formats::header::MessageClass::Request => match message.message_method().value() {
                stun_formats::methods::rfc8489::BINDING::STATIC_VALUE => {
                    let response = Message::builder()
                        .success()
                        .binding()
                        .transaction_id(transaction_id)
                        .attribute(XorMappedAddressAttribute::new(remote))
                        .unwrap()
                        .finger_print()
                        .unwrap()
                        .build()
                        .unwrap();
                    Ok(AgentEvent::OutgoingMessage(response))
                }
                _ => Ok(AgentEvent::FurtherProcess((message, remote))),
            },
            stun_formats::header::MessageClass::Indication => {
                Ok(AgentEvent::FurtherProcess((message, remote)))
            }
            _ => {
                if let Some(entry) = self.transactions.remove(message.transaction_id()) {
                    tracing::debug!(
                        "corresponding transaction: {:?}, retransmit times: {}",
                        entry.request,
                        entry.retransmit_times
                    );
                    return Ok(AgentEvent::OutgoingMessage(message));
                }
                Err(report!(StunSessionError::UnknownTransaction(message)).into_dynamic())
                    .operation("matching response to transaction")
                    .resource("transaction", transaction_id)
            }
        }
    }

    fn on_send_request(&mut self, message: Message) -> StunSessionResult<()> {
        self.transactions.remove(message.transaction_id());
        let now = Instant::now();
        let transaction = Transaction {
            request: message.clone(),
            deadline: now + self.rto,
            retransmit_times: 1,
            transation_start: now,
        };
        self.transactions
            .insert(*message.transaction_id(), transaction);
        tracing::debug!("new transaction with requst:\n{:?}", message);
        Ok(())
    }

    async fn process_events(&mut self, events: Vec<AgentEvent>) -> StunSessionResult<()> {
        for event in events {
            match self.event_tx.send_timeout(event, self.timer_interval).await {
                Ok(()) => {}
                Err(err) => {
                    tracing::error!("stun agent send event out timeout: {}", err);
                }
            }
        }
        Ok(())
    }

    pub async fn run(mut self) -> StunSessionResult<()> {
        let mut timer = tokio::time::interval(self.timer_interval);
        loop {
            tokio::select! {
                Some(cmd) = self.command_rx.recv() => match cmd {
                    AgentCommand::IncomingMessage(msg) => {
                        let event = self.on_incoming_message(msg)
                            .operation("processing incoming STUN command")?;
                        self.process_events(vec![event]).await?;
                    }
                    AgentCommand::SendRequest(req) => {
                        self.on_send_request(req)?;
                    }
                },
                _ = timer.tick() => {
                    let events = self.on_timer()?;
                    self.process_events(events).await?;
                }
                else => {
                    tracing::debug!("stun agent command channel has been closed, exiting");
                    return Ok(());
                }
            }
        }
    }
}
