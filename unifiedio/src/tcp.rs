use std::task::Poll;

use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};

use crate::UnifiedIO;

#[derive(Debug)]
pub struct TcpIO {
    inner: TcpStream,
}

impl TcpIO {
    pub fn new(inner: TcpStream) -> Self {
        Self { inner }
    }
}

impl UnifiedIO for TcpIO {
    fn get_underlying_io(&self) -> crate::UnderlyingIO {
        crate::UnderlyingIO::TCP {
            local_addr: self.inner.local_addr().ok(),
            peer_addr: self.inner.peer_addr().ok(),
        }
    }
}

impl AsyncRead for TcpIO {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.inner.poll_read_ready(cx) {
            Poll::Ready(Ok(())) => match self.inner.try_read_buf(buf) {
                Ok(_) => Poll::Ready(Ok(())),
                Err(err) => Poll::Ready(Err(err)),
            },
            Poll::Ready(Err(err)) => Poll::Ready(Err(err)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl AsyncWrite for TcpIO {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        tokio::io::AsyncWrite::poll_write(std::pin::Pin::new(&mut self.get_mut().inner), cx, buf)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        tokio::io::AsyncWrite::poll_flush(std::pin::Pin::new(&mut self.get_mut().inner), cx)
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        tokio::io::AsyncWrite::poll_shutdown(std::pin::Pin::new(&mut self.get_mut().inner), cx)
    }
}
