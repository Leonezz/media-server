use crate::{
    UnifiedIO,
    errors::{UnifiedIOError, UnifiedIOResult},
};
use futures::{Sink, Stream, ready};
use std::{
    io::{self},
    net::SocketAddr,
    task::Poll,
};
use tokio::net::UdpSocket;
use tokio_util::bytes::Bytes;

#[derive(Debug)]
pub struct UdpIO {
    inner: UdpSocket,
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
    pending_send: Option<Bytes>,
}

impl UdpIO {
    pub async fn new(local_addr: SocketAddr, remote_addr: SocketAddr) -> UnifiedIOResult<Self> {
        match UdpSocket::bind(local_addr).await {
            Ok(socket) => match socket.connect(remote_addr).await {
                Ok(_) => Ok(Self {
                    local_addr: socket.local_addr().unwrap(),
                    peer_addr: socket.peer_addr().unwrap(),
                    inner: socket,
                    pending_send: None,
                }),
                Err(err) => Err(UnifiedIOError::Io(err)),
            },
            Err(err) => Err(UnifiedIOError::Io(err)),
        }
    }

    pub async fn new_with_remote_addr(
        local_port_start_from: u16,
        remote_addr: SocketAddr,
    ) -> UnifiedIOResult<(u16, Self)> {
        for port in (local_port_start_from..=u16::MAX).step_by(1) {
            let local_addr =
                SocketAddr::new(std::net::IpAddr::V4("0.0.0.0".parse().unwrap()), port);
            match Self::new(local_addr, remote_addr).await {
                Ok(io) => return Ok((port, io)),
                Err(err) => {
                    tracing::warn!("failed to bind to port {}: {:?}", port, err);
                }
            }
        }
        Err(UnifiedIOError::Io(std::io::Error::other(
            "Failed to bind to any port",
        )))
    }
}

impl UnifiedIO for UdpIO {
    fn get_underlying_io_type(&self) -> crate::UnderlyingIO {
        crate::UnderlyingIO::UDP {
            local_addr: Some(self.local_addr),
            peer_addr: Some(self.peer_addr),
        }
    }
}

impl Sink<Bytes> for UdpIO {
    type Error = std::io::Error;
    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.poll_flush(cx)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        if let Some(bytes) = self.pending_send.take() {
            match self.inner.poll_send(cx, &bytes) {
                Poll::Ready(Ok(_len)) => Poll::Ready(Ok(())),
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Pending => {
                    self.pending_send = Some(bytes);
                    Poll::Pending
                }
            }
        } else {
            // No pending item, nothing to flush.
            Poll::Ready(Ok(()))
        }
    }

    fn poll_ready(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        if self.pending_send.is_some() {
            ready!(self.as_mut().poll_flush(cx))?;
            assert!(self.pending_send.is_none());
            Poll::Ready(Ok(()))
        } else {
            Poll::Ready(Ok(()))
        }
    }
    fn start_send(mut self: std::pin::Pin<&mut Self>, item: Bytes) -> Result<(), Self::Error> {
        if self.pending_send.is_some() {
            return Err(io::Error::other(
                "UDP sink not ready, previous send still pending",
            ));
        }
        self.pending_send = Some(item);
        Ok(())
    }
}

impl Stream for UdpIO {
    type Item = Result<Bytes, std::io::Error>;
    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut recv_buffer = vec![0; 4096];
        let mut recv_slice = tokio::io::ReadBuf::new(&mut recv_buffer);
        match self.inner.poll_recv(cx, &mut recv_slice) {
            Poll::Ready(Ok(_)) => {
                let data = Bytes::copy_from_slice(recv_slice.filled());
                Poll::Ready(Some(Ok(data)))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            Poll::Pending => Poll::Pending,
        }
    }
}
