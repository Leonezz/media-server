use crate::errors::AppError;
use std::net::IpAddr;
use utils::net::protocol::Protocol;

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
