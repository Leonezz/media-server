use std::{
    fmt::Debug,
    net::SocketAddr,
    pin::Pin,
    task::{Poll, ready},
};

use futures::{Sink, SinkExt, Stream, StreamExt};
use tokio_util::{
    bytes::{Bytes, BytesMut},
    codec::{Decoder, Encoder},
};
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

pub trait UnifiedIO:
    Stream<Item = Result<Bytes, std::io::Error>> + Sink<Bytes, Error = std::io::Error> + Debug + Send
{
    fn get_underlying_io_type(&self) -> UnderlyingIO;
    fn get_local_addr(&self) -> Option<SocketAddr> {
        match self.get_underlying_io_type() {
            UnderlyingIO::TCP { local_addr, .. } => local_addr,
            UnderlyingIO::UDP { local_addr, .. } => local_addr,
            UnderlyingIO::Channel => None,
        }
    }
    fn get_peer_addr(&self) -> Option<SocketAddr> {
        match self.get_underlying_io_type() {
            UnderlyingIO::TCP { peer_addr, .. } => peer_addr,
            UnderlyingIO::UDP { peer_addr, .. } => peer_addr,
            UnderlyingIO::Channel => None,
        }
    }
}

const INITIAL_RD_CAPACITY: usize = 64 * 1024;
pub struct UnifiyStreamed<C> {
    io: Pin<Box<dyn UnifiedIO>>,
    read_buffer: BytesMut,
    is_readable: bool,
    codec: C,
}

impl<C> UnifiyStreamed<C> {
    pub fn new(io: Pin<Box<dyn UnifiedIO>>, codec: C) -> Self {
        Self {
            io,
            read_buffer: BytesMut::new(),
            is_readable: false,
            codec,
        }
    }
}

impl<C> Unpin for UnifiyStreamed<C> {}

impl<C> Stream for UnifiyStreamed<C>
where
    C: Decoder,
{
    type Item = Result<C::Item, C::Error>;
    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let pin = self.get_mut();
        pin.read_buffer.reserve(INITIAL_RD_CAPACITY);
        loop {
            if pin.is_readable {
                if let Some(frame) = pin.codec.decode_eof(&mut pin.read_buffer)? {
                    return Poll::Ready(Some(Ok(frame)));
                }
            }
            pin.is_readable = false;
            pin.read_buffer.clear();

            let res = ready!(pin.io.poll_next_unpin(cx));
            if res.is_none() {
                return Poll::Ready(None);
            }
            pin.read_buffer.extend(res.unwrap()?);
            pin.is_readable = true;
        }
    }
}

impl<I, C> Sink<I> for UnifiyStreamed<C>
where
    C: Encoder<I>,
{
    type Error = C::Error;
    fn poll_close(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.io.poll_close_unpin(cx).map_err(|err| err.into())
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.io.poll_flush_unpin(cx).map_err(|err| err.into())
    }

    fn poll_ready(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        self.io.poll_ready_unpin(cx).map_err(|err| err.into())
    }

    fn start_send(self: Pin<&mut Self>, item: I) -> Result<(), Self::Error> {
        let pin = self.get_mut();
        let mut bytes = BytesMut::new();
        pin.codec.encode(item, &mut bytes)?;
        pin.io
            .start_send_unpin(bytes.freeze())
            .map_err(|err| err.into())
    }
}
