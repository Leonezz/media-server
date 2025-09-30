use std::{net::IpAddr, str::FromStr};

use crate::errors::AppError;

#[derive(Debug, Clone, Copy)]
pub enum Protocol {
    Tcp,
    Udp,
}

impl FromStr for Protocol {
    type Err = AppError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tcp" => Ok(Self::Tcp),
            "udp" => Ok(Self::Udp),
            _ => Err(AppError::UnknownProtocol(s.to_owned())),
        }
    }
}

impl From<Protocol> for &'static str {
    fn from(value: Protocol) -> Self {
        match value {
            Protocol::Tcp => "tcp",
            Protocol::Udp => "udp",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub server: String,
    pub protocol: Protocol,
    pub local_addr: IpAddr,
    pub local_port: u16,
}

impl AppConfig {
    pub fn validate(&self) -> Result<(), AppError> {
        Ok(())
    }
}
