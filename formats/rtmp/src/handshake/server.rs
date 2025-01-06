use core::time;
use std::{
    fmt::Debug,
    io::Read,
    time::{SystemTime, UNIX_EPOCH},
};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio_util::{
    bytes::{Buf, BytesMut},
    codec::Encoder,
    either::Either,
};

use super::{
    C0S0Packet, C1S1Packet, C2S2Packet, HandshakeServerState, RTMP_VERSION,
    codec::{C0S0PacketCodec, C1S1PacketCodec, C2S2PacketCodec},
    consts::{RTMP_HANDSHAKE_SIZE, RTMP_SERVER_KEY, RTMP_SERVER_VERSION, SHA256_DIGEST_SIZE},
    digest::{make_digest, make_message, validate_c1_digest},
    errors::{DigestError, HandshakeError, HandshakeResult},
};

pub trait AsyncHandshakeServer: Send {
    fn read_c0(&mut self) -> impl std::future::Future<Output = HandshakeResult<()>> + Send;
    fn read_c1(&mut self) -> impl std::future::Future<Output = HandshakeResult<()>> + Send;
    fn read_c2(&mut self) -> impl std::future::Future<Output = HandshakeResult<()>> + Send;

    fn write_s0(&mut self) -> impl std::future::Future<Output = HandshakeResult<()>> + Send;
    fn write_s1(&mut self) -> impl std::future::Future<Output = HandshakeResult<()>> + Send;
    fn write_s2(&mut self) -> impl std::future::Future<Output = HandshakeResult<()>> + Send;
    fn flush(&mut self) -> impl std::future::Future<Output = HandshakeResult<()>> + Send;

    fn state(&self) -> HandshakeServerState;
    fn set_state(&mut self, state: HandshakeServerState);

    fn handshake(&mut self) -> impl std::future::Future<Output = HandshakeResult<()>> + Send {
        async {
            loop {
                let state = self.state();
                tracing::debug!("handshake with state: {:?}", state);
                match state {
                    HandshakeServerState::Uninitialized => {
                        self.read_c0().await?;
                        self.read_c1().await?;
                        self.set_state(HandshakeServerState::C0C1Recived);
                    }
                    HandshakeServerState::C0C1Recived => {
                        self.write_s0().await?;
                        self.write_s1().await?;
                        self.write_s2().await?;
                        self.flush().await?;
                        self.set_state(HandshakeServerState::S0S1S2Sent);
                    }
                    HandshakeServerState::S0S1S2Sent => {
                        self.read_c2().await?;
                        self.set_state(HandshakeServerState::Done);
                    }
                    HandshakeServerState::Done => break,
                }
            }
            Ok(())
        }
    }
}

#[derive(Debug)]
struct SimpleHandshakeServer<T: AsyncRead + AsyncWrite + Unpin + Debug + Send> {
    io: T,
    c1_bytes: Vec<u8>,
    c1_timestamp: u32,
    state: HandshakeServerState,
}

impl<T> SimpleHandshakeServer<T>
where
    T: AsyncRead + AsyncWrite + Unpin + Debug + Send,
{
    pub fn _new(io: T) -> Self {
        Self {
            io,
            c1_bytes: Vec::with_capacity(1536),
            c1_timestamp: 0,
            state: HandshakeServerState::Uninitialized,
        }
    }
}

unsafe impl<T> Send for SimpleHandshakeServer<T> where
    T: Send + AsyncRead + AsyncWrite + Unpin + Debug
{
}

impl<IO> AsyncHandshakeServer for SimpleHandshakeServer<IO>
where
    IO: AsyncRead + AsyncWrite + Unpin + Debug + Send,
{
    async fn flush(&mut self) -> HandshakeResult<()> {
        self.io.flush().await?;
        Ok(())
    }
    fn state(&self) -> HandshakeServerState {
        self.state.clone()
    }
    fn set_state(&mut self, state: HandshakeServerState) {
        self.state = state
    }
    async fn read_c0(&mut self) -> HandshakeResult<()> {
        self.io.read_u8().await?;
        tracing::debug!("read c0");
        Ok(())
    }
    async fn read_c1(&mut self) -> HandshakeResult<()> {
        self.c1_bytes.resize(RTMP_HANDSHAKE_SIZE, 0);
        let len = self.io.read_exact(&mut self.c1_bytes).await?;
        tracing::debug!("read c1, {}", len);
        Ok(())
    }
    async fn read_c2(&mut self) -> HandshakeResult<()> {
        let mut buf: [u8; RTMP_HANDSHAKE_SIZE] = [0; RTMP_HANDSHAKE_SIZE];
        self.io.read_exact(&mut buf).await?;
        tracing::debug!("read c2");
        Ok(())
    }
    async fn write_s0(&mut self) -> HandshakeResult<()> {
        let mut bytes = BytesMut::with_capacity(1);
        C0S0PacketCodec.encode(
            C0S0Packet {
                version: RTMP_VERSION,
            },
            &mut bytes,
        )?;
        self.io.write_all(&bytes).await?;
        self.io.flush().await?;
        tracing::debug!("s0 bytes sent");
        Ok(())
    }
    async fn write_s1(&mut self) -> HandshakeResult<()> {
        let mut bytes = BytesMut::with_capacity(RTMP_HANDSHAKE_SIZE);
        let mut random_bytes: [u8; 1528] = [0; 1528];
        utils::random::random_fill(&mut random_bytes);
        C1S1PacketCodec.encode(
            C1S1Packet {
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?,
                _zeros: self.c1_timestamp,
                random_bytes,
            },
            &mut bytes,
        )?;
        self.io.write_all(&bytes).await?;
        self.io.flush().await?;
        tracing::debug!("s1 bytes sent, {:?}", self.io);
        Ok(())
    }
    async fn write_s2(&mut self) -> HandshakeResult<()> {
        self.io.write_all(&self.c1_bytes).await?;
        self.io.flush().await?;
        tracing::debug!("s2 bytes sent");
        Ok(())
    }
}

#[derive(Debug)]
struct ComplexHandshakeServer<T> {
    io: T,
    writer_buffer: BytesMut,
    c1_digest: [u8; SHA256_DIGEST_SIZE],
    c1_bytes: Vec<u8>,
    c1_timestamp: u32,
    state: HandshakeServerState,
}

impl<T> ComplexHandshakeServer<T>
where
    T: AsyncRead + AsyncWrite + Unpin + Debug + Send,
{
    pub fn new(io: T) -> Self {
        Self {
            io,
            writer_buffer: BytesMut::with_capacity(4096),
            c1_digest: [0; SHA256_DIGEST_SIZE],
            c1_bytes: Vec::with_capacity(RTMP_HANDSHAKE_SIZE),
            c1_timestamp: 0,
            state: HandshakeServerState::Uninitialized,
        }
    }
}

impl<T> AsyncHandshakeServer for ComplexHandshakeServer<T>
where
    T: AsyncRead + AsyncWrite + Unpin + Debug + Send + Send,
{
    async fn flush(&mut self) -> HandshakeResult<()> {
        self.io.write_all(&self.writer_buffer[..]).await?;
        self.io.flush().await?;
        self.writer_buffer.clear();
        Ok(())
    }
    fn state(&self) -> HandshakeServerState {
        self.state.clone()
    }
    fn set_state(&mut self, state: HandshakeServerState) {
        self.state = state
    }
    async fn read_c0(&mut self) -> HandshakeResult<()> {
        self.io.read_u8().await?;
        tracing::debug!("read c0");
        Ok(())
    }
    async fn read_c1(&mut self) -> HandshakeResult<()> {
        self.c1_bytes.resize(RTMP_HANDSHAKE_SIZE, 0);
        let len = self.io.read_exact(&mut self.c1_bytes).await?;
        tracing::debug!("read c1, {}", len);
        let mut bytes = [0_u8; RTMP_HANDSHAKE_SIZE];
        bytes.copy_from_slice(&self.c1_bytes);

        let digest = validate_c1_digest(&bytes)?;
        if digest.len() != SHA256_DIGEST_SIZE {
            return Err(HandshakeError::DigestError(DigestError::WrongLength {
                length: digest.len(),
            }));
        }
        self.c1_digest.copy_from_slice(&digest);
        tracing::debug!("c1 validate success");
        Ok(())
    }
    async fn read_c2(&mut self) -> HandshakeResult<()> {
        let mut bytes = [0_u8; RTMP_HANDSHAKE_SIZE];
        self.io.read_exact(&mut bytes).await?;
        tracing::debug!("read c2");
        Ok(())
    }
    async fn write_s0(&mut self) -> HandshakeResult<()> {
        C0S0PacketCodec.encode(
            C0S0Packet {
                version: RTMP_VERSION,
            },
            &mut self.writer_buffer,
        )?;
        tracing::debug!("write s0");
        Ok(())
    }
    async fn write_s1(&mut self) -> HandshakeResult<()> {
        let mut bytes = BytesMut::with_capacity(RTMP_HANDSHAKE_SIZE);
        let mut random_bytes: [u8; 1528] = [0; 1528];

        utils::random::random_fill(&mut random_bytes);
        C1S1PacketCodec.encode(
            C1S1Packet {
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap(),
                _zeros: RTMP_SERVER_VERSION.into(),
                random_bytes,
            },
            &mut bytes,
        )?;
        let mut bytes_array: [u8; 1536] = [0; RTMP_HANDSHAKE_SIZE];
        bytes.reader().read_exact(&mut bytes_array)?;
        let message = make_message(&RTMP_SERVER_KEY, &bytes_array)?;
        self.writer_buffer.extend_from_slice(&message);
        tracing::debug!("write s1");
        Ok(())
    }
    async fn write_s2(&mut self) -> HandshakeResult<()> {
        let mut bytes = BytesMut::with_capacity(RTMP_HANDSHAKE_SIZE);
        let mut random_bytes: [u8; 1528] = [0; 1528];

        utils::random::random_fill(&mut random_bytes);
        C2S2PacketCodec.encode(
            C2S2Packet {
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap(),
                timestamp2: time::Duration::from_millis(self.c1_timestamp as u64),
                random_echo: random_bytes,
            },
            &mut bytes,
        )?;
        let key = make_digest(&RTMP_SERVER_KEY, &self.c1_digest)?;
        let mut bytes_array: [u8; RTMP_HANDSHAKE_SIZE] = [0; RTMP_HANDSHAKE_SIZE];
        bytes.reader().read_exact(&mut bytes_array)?;
        let digest = make_digest(
            &key,
            &bytes_array[..RTMP_HANDSHAKE_SIZE - SHA256_DIGEST_SIZE],
        )?;
        self.writer_buffer.extend_from_slice(
            [
                &bytes_array[..RTMP_HANDSHAKE_SIZE - SHA256_DIGEST_SIZE],
                &digest[..],
            ]
            .concat()
            .as_slice(),
        );
        tracing::debug!("write s2");
        Ok(())
    }
}

impl<T> From<ComplexHandshakeServer<T>> for SimpleHandshakeServer<T>
where
    T: AsyncRead + AsyncWrite + Unpin + Debug + Send,
{
    fn from(value: ComplexHandshakeServer<T>) -> Self {
        SimpleHandshakeServer {
            io: value.io,
            c1_bytes: value.c1_bytes,
            c1_timestamp: 0,
            state: value.state,
        }
    }
}

#[derive(Debug)]
pub struct HandshakeServer<T: AsyncRead + AsyncWrite + Unpin + Debug + Send> {
    handshaker: Either<ComplexHandshakeServer<T>, SimpleHandshakeServer<T>>,
}

impl<T> HandshakeServer<T>
where
    T: AsyncRead + AsyncWrite + Unpin + Debug + Send,
{
    pub fn new(io: T) -> Self {
        Self {
            handshaker: Either::Left(ComplexHandshakeServer::new(io)),
        }
    }

    pub async fn handshake(mut self, complex_only: bool) -> HandshakeResult<()> {
        if let Either::Left(mut h) = self.handshaker {
            tracing::debug!("now do complex handshake");
            match h.handshake().await {
                Err(HandshakeError::DigestError(err)) => {
                    if complex_only {
                        return Err(HandshakeError::DigestError(err));
                    }
                    tracing::trace!(
                        "complex handshake failed due to digest error: {}, retry with simple handshake",
                        err
                    );
                    let mut sim: SimpleHandshakeServer<_> = h.into();
                    sim.state = HandshakeServerState::C0C1Recived;
                    self.handshaker = Either::Right(sim);
                }
                Err(err) => {
                    tracing::error!("complex handshake failed: {}", err);
                    return Err(err);
                }
                Ok(()) => return Ok(()),
            }
        }

        if let Either::Right(mut h) = self.handshaker {
            tracing::debug!("now do simple handshake");
            h.handshake().await?;
        }

        Ok(())
    }
}
