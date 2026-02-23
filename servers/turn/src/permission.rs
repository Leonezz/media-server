use std::{
    net::IpAddr,
    time::{Duration, Instant},
};

#[derive(Debug, Clone)]
pub struct Permission {
    peer_ip: IpAddr,
    time_to_expiry: Instant,
}

pub const PERMISSION_LIFETIME: Duration = Duration::from_mins(5);

impl Permission {
    pub fn new(ip: IpAddr, time_to_expiry: Instant) -> Self {
        Self {
            peer_ip: ip,
            time_to_expiry,
        }
    }

    pub fn expired(&self) -> bool {
        Instant::now() > self.time_to_expiry
    }

    pub fn refresh(&mut self) {
        let time_to_expiry = Instant::now().checked_add(PERMISSION_LIFETIME).unwrap();
        self.time_to_expiry = time_to_expiry;
    }

    pub fn peer_ip(&self) -> IpAddr {
        self.peer_ip
    }
}
