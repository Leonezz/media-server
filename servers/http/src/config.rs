use std::net::IpAddr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct HttpServerConfig {
    // ip address to serve on
    pub address: IpAddr,
    // port to serve on
    pub port: u16,
    // number of threads to use for executing futures
    pub workers: u64,
}
