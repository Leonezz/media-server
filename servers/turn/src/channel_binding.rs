use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};

#[derive(Debug, Clone)]
pub struct ChannelBinding {
    channel_number: u16,
    peer_transport_address: SocketAddr,
    time_to_expiry: Instant,
}

pub const CHANNEL_BINDING_LIFETIME: Duration = Duration::from_mins(10);

impl ChannelBinding {
    pub fn new(channel_number: u16, peer: SocketAddr, time_to_expiry: Instant) -> Self {
        Self {
            channel_number,
            peer_transport_address: peer,
            time_to_expiry,
        }
    }

    pub fn expired(&self) -> bool {
        Instant::now() > self.time_to_expiry
    }

    pub fn refresh(&mut self) {
        let time_to_expiry = Instant::now()
            .checked_add(CHANNEL_BINDING_LIFETIME)
            .unwrap();
        self.time_to_expiry = time_to_expiry;
    }

    pub fn peer(&self) -> SocketAddr {
        self.peer_transport_address
    }

    pub fn channel_number(&self) -> u16 {
        self.channel_number
    }
}
