use rtp_formats::{
    errors::RtpResult, packet::RtpTrivialPacket, rtcp::compound_packet::RtcpCompoundPacket,
};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

#[derive(Debug)]
pub enum EventToRtpSession {
    RtpPacketToSend(RtpTrivialPacket),
    SetCname(String),
    Subscribe {
        result_sender: oneshot::Sender<RtpResult<SubscribeResponse>>,
    },
    Unsubscribe {
        id: Uuid,
    },
    StopSession(String),
}

pub type EventToRtpSessionSender = mpsc::Sender<EventToRtpSession>;
pub type EventToRtpSessionReceiver = mpsc::Receiver<EventToRtpSession>;

#[derive(Debug, Clone)]
pub enum EventFromRtpSession {
    RtpPacketReceived(RtpTrivialPacket),
    RtcpPacketReceived(RtcpCompoundPacket),
    SessionClosed(Option<String>),
}

#[derive(Debug)]
pub struct SubscribeResponse {
    pub event_receiver: mpsc::Receiver<EventFromRtpSession>,
    pub id: Uuid,
}

#[derive(Debug)]
pub struct RtpSessionEventProducer {
    pub event_sender: mpsc::Sender<EventFromRtpSession>,
    pub id: Uuid,
}
