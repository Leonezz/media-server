use futures::{Sink, Stream};
use pin_project::pin_project;
use std::{
    fmt, io,
    pin::Pin,
    task::{Context, Poll, ready},
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::oneshot,
};
use tokio_util::{
    codec::{Decoder, Encoder, Framed},
    sync::PollSender,
};

use crate::io::WakerSource;

type Ack = oneshot::Sender<()>;
pub type Outgoing<T> = (T, Option<Ack>);

#[derive(Debug, Default)]
enum ReadState {
    #[default]
    Reading,
    Done,
}

#[derive(Debug, Default)]
enum WriteState {
    #[default]
    Writing,
    Flushing((Option<Ack>, FromFlushingTo)),
    ShuttingDown,
    Done,
}

#[derive(Debug, Clone, Copy)]
enum FromFlushingTo {
    Writing,
    ShuttingDown,
}

impl From<FromFlushingTo> for WriteState {
    fn from(status: FromFlushingTo) -> Self {
        match status {
            FromFlushingTo::Writing => Self::Writing,
            FromFlushingTo::ShuttingDown => Self::ShuttingDown,
        }
    }
}

impl WriteState {
    fn set_flushing(&mut self, ack: Option<Ack>, next: FromFlushingTo) {
        if !matches!(self, WriteState::Writing) {
            unreachable!("bug set_flushing");
        }

        *self = WriteState::Flushing((ack, next));
    }
}

#[derive(Debug)]
#[pin_project]
pub struct Connection<IO, C, E, In, Out>
where
    IO: AsyncRead + AsyncWrite + WakerSource + Unpin,
    E: From<std::io::Error>,
    C: Encoder<Out, Error = E> + Decoder<Item = In, Error = E>,
    In: Send + fmt::Debug,
    Out: Send + fmt::Debug,
{
    #[pin]
    stream: Framed<IO, C>,

    read_buffer: Option<In>,
    tx: PollSender<In>,

    write_buffer: Option<Outgoing<Out>>,
    rx: tokio::sync::mpsc::Receiver<Outgoing<Out>>,

    read_state: ReadState,
    write_state: WriteState,
}

impl<IO, C, E, In, Out> Connection<IO, C, E, In, Out>
where
    IO: AsyncRead + AsyncWrite + WakerSource + Unpin,
    E: From<std::io::Error>,
    C: Encoder<Out, Error = E> + Decoder<Item = In, Error = E>,
    In: Send + fmt::Debug,
    Out: Send + fmt::Debug,
{
    pub fn new(
        io: IO,
        codec: C,
    ) -> (
        Self,
        tokio::sync::mpsc::Sender<Outgoing<Out>>,
        tokio::sync::mpsc::Receiver<In>,
    ) {
        let (read_tx, read_rx) = tokio::sync::mpsc::channel(1000);
        let (write_tx, write_rx) = tokio::sync::mpsc::channel(1000);
        let stream = Framed::new(io, codec);
        let conn = Self {
            stream,
            read_buffer: None,
            tx: PollSender::new(read_tx),
            write_buffer: None,
            rx: write_rx,
            read_state: ReadState::default(),
            write_state: WriteState::default(),
        };
        (conn, write_tx, read_rx)
    }

    fn poll_read(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), E>> {
        if !matches!(self.read_state, ReadState::Reading) {
            unreachable!("poll_read called in invalid state");
        }

        loop {
            if self.read_buffer.is_some() {
                ready!(
                    self.tx
                        .poll_reserve(cx)
                        .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "Channel closed"))?
                );
                let item = self.read_buffer.take().expect("buffered message missing");
                self.tx
                    .send_item(item)
                    .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "Channel closed"))?;
            }

            let mut stream = Pin::new(&mut self.stream);
            match ready!(Stream::poll_next(stream.as_mut(), cx)) {
                Some(Ok(item)) => {
                    self.read_buffer = Some(item);
                }
                Some(Err(err)) => {
                    return Poll::Ready(Err(err));
                }
                None => {
                    return Poll::Pending;
                }
            }
        }
    }

    fn poll_write(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), E>> {
        if !matches!(self.write_state, WriteState::Writing) {
            unreachable!("poll_write called in invalid state");
        }

        loop {
            if self.write_buffer.is_none() {
                let msg = match self.rx.poll_recv(cx) {
                    Poll::Ready(Some(msg)) => msg,
                    Poll::Ready(None) => {
                        self.write_state
                            .set_flushing(None, FromFlushingTo::ShuttingDown);
                        return Poll::Ready(Ok(()));
                    }
                    Poll::Pending => {
                        self.write_state.set_flushing(None, FromFlushingTo::Writing);
                        return Poll::Pending;
                    }
                };
                self.write_buffer = Some(msg);
            }

            if let Some((msg, ack)) = self.write_buffer.take() {
                let mut stream = Pin::new(&mut self.stream);
                match Sink::poll_ready(stream.as_mut(), cx)? {
                    Poll::Ready(()) => {
                        Sink::start_send(stream.as_mut(), msg)?;

                        if let Some(ack) = ack {
                            self.write_state
                                .set_flushing(Some(ack), FromFlushingTo::Writing);
                            cx.waker().wake_by_ref();
                            return Poll::Pending;
                        }
                    }
                    Poll::Pending => {
                        self.write_buffer = Some((msg, ack));
                        return Poll::Pending;
                    }
                }
            }
        }
    }

    fn poll_flush(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), E>> {
        let WriteState::Flushing(state) = &mut self.write_state else {
            unreachable!("poll_flush called in invalid state");
        };

        let mut stream = Pin::new(&mut self.stream);
        match ready!(Sink::poll_flush(stream.as_mut(), cx)) {
            Ok(()) => {
                let (ack, next) = state;

                if let Some(ack) = ack.take() {
                    _ = ack.send(());
                }

                self.write_state = (*next).into();
                Poll::Ready(Ok(()))
            }
            Err(err) => Poll::Ready(Err(err)),
        }
    }

    fn poll_shutdown(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), E>> {
        if !matches!(self.write_state, WriteState::ShuttingDown) {
            unreachable!("poll_shutdown called in invalid state");
        }

        let mut stream = Pin::new(&mut self.stream);
        match ready!(Sink::poll_close(stream.as_mut(), cx)) {
            Ok(()) => {
                self.write_state = WriteState::Done;
                Poll::Ready(Ok(()))
            }
            Err(err) => Poll::Ready(Err(err)),
        }
    }
}

impl<IO, C, E, In, Out> std::future::Future for Connection<IO, C, E, In, Out>
where
    IO: AsyncRead + AsyncWrite + WakerSource + Unpin,
    E: From<std::io::Error>,
    C: Decoder<Item = In, Error = E> + Encoder<Out, Error = E>,
    In: Send + fmt::Debug,
    Out: Send + fmt::Debug,
{
    type Output = Result<(), E>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if matches!(self.write_state, WriteState::Writing) {
            _ = self.poll_write(cx)?;
        }

        if matches!(self.write_state, WriteState::Flushing(_)) {
            _ = self.poll_flush(cx)?;
        }

        if matches!(self.write_state, WriteState::ShuttingDown) {
            _ = self.poll_shutdown(cx)?;
        }

        if matches!(self.read_state, ReadState::Done)
            && matches!(self.write_state, WriteState::Done)
        {
            return Poll::Ready(Ok(()));
        }
        if matches!(self.read_state, ReadState::Reading) {
            _ = self.poll_read(cx)?;
        }

        self.stream.get_mut().register_waker(cx.waker());
        Poll::Pending
    }
}
