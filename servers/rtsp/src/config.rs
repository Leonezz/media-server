use std::net::IpAddr;

#[derive(Debug)]
pub struct RtspServerConfig {
    pub address: IpAddr,
    pub port: u16,
}
