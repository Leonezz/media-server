use std::{net::SocketAddr, task::Poll};

use futures::{Sink, SinkExt, Stream, StreamExt, ready};
use std::pin::Pin;
use tokio::net::TcpStream;
use tokio_util::{
    bytes::Bytes,
    codec::{BytesCodec, Framed},
};

use crate::UnifiedIO;

#[derive(Debug)]
pub struct TcpIO {
    inner: Framed<TcpStream, BytesCodec>,
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
}

impl TcpIO {
    pub fn new(inner: TcpStream) -> Self {
        Self {
            local_addr: inner.local_addr().unwrap(),
            peer_addr: inner.peer_addr().unwrap(),
            inner: Framed::new(inner, BytesCodec::new()),
        }
    }
}

impl UnifiedIO for TcpIO {
    fn get_underlying_io_type(&self) -> crate::UnderlyingIO {
        crate::UnderlyingIO::TCP {
            local_addr: Some(self.local_addr),
            peer_addr: Some(self.peer_addr),
        }
    }
}

impl Sink<Bytes> for TcpIO {
    type Error = std::io::Error;
    fn start_send(mut self: std::pin::Pin<&mut Self>, item: Bytes) -> Result<(), Self::Error> {
        self.inner.start_send_unpin(item)
    }
    fn poll_ready(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        <tokio_util::codec::Framed<tokio::net::TcpStream, tokio_util::codec::BytesCodec> as futures::SinkExt<Bytes>>::poll_ready_unpin(&mut self.inner, cx)
    }
    fn poll_close(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        <tokio_util::codec::Framed<tokio::net::TcpStream, tokio_util::codec::BytesCodec> as futures::SinkExt<Bytes>>::poll_close_unpin(&mut self.inner, cx)
    }
    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        <tokio_util::codec::Framed<tokio::net::TcpStream, tokio_util::codec::BytesCodec> as futures::SinkExt<Bytes>>::poll_flush_unpin(&mut self.inner, cx)
    }
}

impl Stream for TcpIO {
    type Item = Result<Bytes, std::io::Error>;
    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match ready!(self.inner.poll_next_unpin(cx)) {
            Some(Ok(bytes)) => Poll::Ready(Some(Ok(bytes.freeze()))),
            Some(Err(e)) => Poll::Ready(Some(Err(e))),
            None => Poll::Ready(None),
        }
    }
}
