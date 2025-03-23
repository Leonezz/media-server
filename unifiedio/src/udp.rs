use std::{net::SocketAddr, task::Poll};

use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::UdpSocket,
};

use crate::{
    UnifiedIO,
    errors::{UnifiedIOError, UnifiedIOResult},
};

#[derive(Debug)]
pub struct UdpIO {
    inner: UdpSocket,
}

impl UdpIO {
    pub async fn new(local_addr: SocketAddr, remote_addr: SocketAddr) -> UnifiedIOResult<Self> {
        match UdpSocket::bind(local_addr).await {
            Ok(socket) => match socket.connect(remote_addr).await {
                Ok(_) => Ok(Self { inner: socket }),
                Err(err) => Err(UnifiedIOError::Io(err)),
            },
            Err(err) => Err(UnifiedIOError::Io(err)),
        }
    }
}

impl UnifiedIO for UdpIO {
    fn get_underlying_io(&self) -> crate::UnderlyingIO {
        crate::UnderlyingIO::UDP {
            local_addr: self.inner.local_addr().ok(),
            peer_addr: self.inner.peer_addr().ok(),
        }
    }
}

impl AsyncRead for UdpIO {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        self.inner.poll_recv(cx, buf)
    }
}

impl AsyncWrite for UdpIO {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        self.inner.poll_send(cx, buf)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }
}
