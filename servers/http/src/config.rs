use serde::{Deserialize, Serialize};


#[derive(Debug, Deserialize, Serialize, Default, Clone)]
#[serde(crate = "rocket::serde")]
pub struct HttpServerConfig {
    // ip address to serve on
    pub address: String,
    // port to serve on
    pub port: u16,
    // number of threads to use for executing futures
    pub workers: usize,
}
