use std::{fmt::Debug, net::SocketAddr};

use tokio::io::{AsyncRead, AsyncWrite};
pub mod channel;
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
    Channel,
}

pub trait UnifiedIO: AsyncRead + AsyncWrite + Debug + Send {
    fn get_underlying_io(&self) -> UnderlyingIO;
    fn get_local_addr(&self) -> Option<SocketAddr> {
        match self.get_underlying_io() {
            UnderlyingIO::TCP { local_addr, .. } => local_addr,
            UnderlyingIO::UDP { local_addr, .. } => local_addr,
            UnderlyingIO::Channel => None,
        }
    }
    fn get_peer_addr(&self) -> Option<SocketAddr> {
        match self.get_underlying_io() {
            UnderlyingIO::TCP { peer_addr, .. } => peer_addr,
            UnderlyingIO::UDP { peer_addr, .. } => peer_addr,
            UnderlyingIO::Channel => None,
        }
    }
}
