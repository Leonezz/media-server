use tokio::io::{AsyncRead, AsyncWrite};
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
    fn get_underlying_io(&self) -> crate::UnderlyingIO {
        crate::UnderlyingIO::Channel
    }
}

impl AsyncWrite for ChannelIo {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        match self.sink.poll_reserve(cx) {
            std::task::Poll::Pending => return std::task::Poll::Pending,
            std::task::Poll::Ready(Ok(_)) => {
                self.sink
                    .send_item(Bytes::copy_from_slice(buf))
                    .map_err(|_| std::io::Error::other("Failed to send data to the sink"))?;
            }
            std::task::Poll::Ready(Err(_)) => {
                return std::task::Poll::Ready(Err(std::io::ErrorKind::Other.into()));
            }
        }

        std::task::Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        self.sink.close();
        std::task::Poll::Ready(Ok(()))
    }
}

impl AsyncRead for ChannelIo {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        if let Ok(data) = self.source.try_recv() {
            buf.put_slice(&data);
            std::task::Poll::Ready(Ok(()))
        } else {
            cx.waker().wake_by_ref();
            std::task::Poll::Pending
        }
    }
}
