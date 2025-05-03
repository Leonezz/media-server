use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Sink, SinkExt, Stream, ready};
use tokio_util::{bytes::Bytes, sync::PollSender};

use crate::UnifiedIO;

#[derive(Debug)]
pub struct ChannelIo {
    pub(crate) source: tokio::sync::mpsc::Receiver<Bytes>,
    pub(crate) sink: PollSender<Bytes>,
}

impl ChannelIo {
    pub fn new(
        source: tokio::sync::mpsc::Receiver<Bytes>,
        sink: tokio::sync::mpsc::Sender<Bytes>,
    ) -> Self {
        Self {
            source,
            sink: PollSender::new(sink),
        }
    }
}

impl UnifiedIO for ChannelIo {
    fn get_underlying_io_type(&self) -> crate::UnderlyingIO {
        crate::UnderlyingIO::Channel
    }
}

impl Sink<Bytes> for ChannelIo {
    type Error = std::io::Error;
    fn start_send(mut self: std::pin::Pin<&mut Self>, item: Bytes) -> Result<(), Self::Error> {
        self.sink
            .start_send_unpin(item)
            .map_err(|err| std::io::Error::other(err.to_string()))
    }
    fn poll_ready(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.sink
            .poll_ready_unpin(cx)
            .map_err(|err| std::io::Error::other(err.to_string()))
    }

    fn poll_close(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.sink
            .poll_close_unpin(cx)
            .map_err(|err| std::io::Error::other(err.to_string()))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
}

impl Stream for ChannelIo {
    type Item = Result<Bytes, std::io::Error>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match ready!(self.source.poll_recv(cx)) {
            Some(bytes) => Poll::Ready(Some(Ok(bytes))),
            None => Poll::Ready(None),
        }
    }
}
