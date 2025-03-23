use std::net::SocketAddr;

use tokio::io::{AsyncRead, AsyncWrite};
mod errors;
pub mod tcp;
pub mod udp;

pub enum UnderlyingIO {
    TCP {
        local_addr: Option<SocketAddr>,
        peer_addr: Option<SocketAddr>,
    },
    UDP {
        local_addr: Option<SocketAddr>,
        peer_addr: Option<SocketAddr>,
    },
}

pub trait UnifiedIO: AsyncRead + AsyncWrite {
    fn get_underlying_io(&self) -> UnderlyingIO;
}
