use std::{collections::HashMap, net::SocketAddr, sync::Arc, task::ready};

use socket2::{Domain, Type};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpListener, TcpSocket, UdpSocket},
    sync::mpsc,
};
use tokio_util::bytes::Bytes;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Tcp,
    Udp,
    Pipe,
}

#[derive(Debug)]
pub enum ClientEndpointIo {
    Tcp(TcpSocket),
    Udp(UdpSocket),
}

impl ClientEndpointIo {
    pub fn new_tcp(local_addr: SocketAddr) -> Result<Self, std::io::Error> {
        let socket = socket2::Socket::new(
            Domain::for_address(local_addr),
            Type::STREAM,
            Some(socket2::Protocol::TCP),
        )?;
        socket.set_reuse_address(true)?;
        socket.set_reuse_port(true)?;
        socket.set_keepalive(true)?;
        socket.set_nonblocking(true)?;
        socket.set_recv_buffer_size(10 * 1024 * 1024)?;
        socket.set_send_buffer_size(10 * 1024 * 1024)?;
        socket.bind(&local_addr.into())?;

        let std_socket: std::net::TcpStream = socket.into();
        let tokio_socket = tokio::net::TcpSocket::from_std_stream(std_socket);
        Ok(Self::Tcp(tokio_socket))
    }

    pub async fn new_udp(local_addr: SocketAddr) -> Result<Self, std::io::Error> {
        let socket = socket2::Socket::new(
            Domain::for_address(local_addr),
            Type::DGRAM,
            Some(socket2::Protocol::UDP),
        )?;
        socket.set_reuse_address(true)?;
        socket.set_reuse_port(true)?;
        socket.set_nonblocking(true)?;
        socket.bind(&local_addr.into())?;

        let std_socket: std::net::UdpSocket = socket.into();
        let tokio_socket = tokio::net::UdpSocket::from_std(std_socket)?;
        Ok(Self::Udp(tokio_socket))
    }

    pub fn local_addr(&self) -> Result<SocketAddr, std::io::Error> {
        match self {
            Self::Tcp(socket) => socket.local_addr(),
            Self::Udp(socket) => socket.local_addr(),
        }
    }

    pub async fn connect(self, remote: SocketAddr) -> std::io::Result<ConnectionIo> {
        let local_addr = self.local_addr()?;
        match self {
            Self::Tcp(socket) => {
                let stream = socket.connect(remote).await?;
                Ok(ConnectionIo::new(
                    ConnectionIoInner::Tcp(stream),
                    remote,
                    local_addr,
                    None,
                ))
            }
            Self::Udp(socket) => {
                socket.connect(remote).await?;
                let (tx, rx) = mpsc::channel(100);
                let socket = Arc::new(socket);
                let client = ConnectionIo::new(
                    ConnectionIoInner::Udp {
                        socket: socket.clone(),
                        bytes_rx: rx,
                        read_buffer: None,
                        peers: None,
                    },
                    remote,
                    local_addr,
                    None,
                );
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 2048];
                    loop {
                        match socket.recv_from(&mut buf).await {
                            Ok((len, addr)) => {
                                if addr != remote {
                                    tracing::warn!(
                                        "received packet from unexpected address: {}, expected: {}",
                                        addr,
                                        remote
                                    );
                                    continue;
                                }
                                let data = Bytes::copy_from_slice(&buf[..len]);
                                if let Err(err) = tx.send(data).await {
                                    tracing::error!(
                                        "error sending data to receiver {} -> {}: {}",
                                        addr,
                                        local_addr,
                                        err
                                    );
                                    break;
                                }
                            }
                            Err(err) => {
                                tracing::error!("error receiving udp packet: {}", err);
                                break;
                            }
                        }
                    }
                });
                Ok(client)
            }
        }
    }
}

#[derive(Debug)]
pub enum ServerEndpointIoInner {
    Tcp(TcpListener),
    Udp {
        socket: Arc<UdpSocket>,
        peers: Arc<tokio::sync::Mutex<HashMap<SocketAddr, mpsc::Sender<Bytes>>>>,
        new_connection_rx: mpsc::Receiver<ConnectionIo>,
    },
}

#[derive(Debug, Default)]
pub struct Shared {
    number_of_connections: usize,
}

#[derive(Debug)]
pub struct ServerEndpointIo {
    inner: ServerEndpointIoInner,
    protocol: Protocol,
    local_addr: SocketAddr,
    shared: Arc<tokio::sync::Mutex<Shared>>,
}

#[derive(Debug)]
pub enum ConnectionIoInner {
    Tcp(tokio::net::TcpStream),
    Udp {
        socket: Arc<tokio::net::UdpSocket>,
        peers: Option<Arc<tokio::sync::Mutex<HashMap<SocketAddr, mpsc::Sender<Bytes>>>>>,
        bytes_rx: mpsc::Receiver<Bytes>,
        read_buffer: Option<Bytes>, // for udp accept new connection
    },
}

#[derive(Debug)]
pub struct ConnectionIo {
    inner: ConnectionIoInner,
    remote_addr: SocketAddr,
    local_addr: SocketAddr,
    bytes_sent: usize,
    bytes_received: usize,
    shared: Arc<tokio::sync::Mutex<Shared>>,
    waker: Option<std::task::Waker>,
}

pub trait WakerSource {
    fn register_waker(&mut self, waker: &std::task::Waker);
}

impl WakerSource for ConnectionIo {
    fn register_waker(&mut self, waker: &std::task::Waker) {
        if let Some(old_waker) = self.waker.as_ref()
            && old_waker.will_wake(waker)
        {
            return;
        }
        self.waker = Some(waker.clone());
    }
}

impl Drop for ConnectionIo {
    fn drop(&mut self) {
        let shared = self.shared.clone();
        let mut peers = match &self.inner {
            ConnectionIoInner::Tcp(_) => None,
            ConnectionIoInner::Udp { peers, .. } => peers.clone(),
        };
        let remote_addr = self.remote_addr;
        let waker = self.waker.take();
        tokio::spawn(async move {
            if let Some(peers) = peers.take() {
                peers.lock().await.remove(&remote_addr);
            }
            let mut shared = shared.lock().await;
            if shared.number_of_connections > 0 {
                shared.number_of_connections -= 1;
            }
            if let Some(waker) = waker {
                waker.wake();
            }
        });
    }
}

impl ServerEndpointIo {
    pub async fn new_tcp(local_addr: SocketAddr) -> std::io::Result<Self> {
        let socket = socket2::Socket::new(
            Domain::for_address(local_addr),
            Type::STREAM,
            Some(socket2::Protocol::TCP),
        )?;
        socket.set_reuse_address(true)?;
        socket.set_reuse_port(true)?;
        socket.set_keepalive(true)?;
        socket.set_nonblocking(true)?;
        socket.set_recv_buffer_size(10 * 1024 * 1024)?;
        socket.set_send_buffer_size(10 * 1024 * 1024)?;
        socket.bind(&local_addr.into())?;
        socket.listen(1280)?;

        let std_listener: std::net::TcpListener = socket.into();
        let listener = TcpListener::from_std(std_listener)?;
        let real_local_addr = listener.local_addr()?;
        Ok(Self {
            inner: ServerEndpointIoInner::Tcp(listener),
            protocol: Protocol::Tcp,
            local_addr: real_local_addr,
            shared: Arc::new(tokio::sync::Mutex::new(Shared::default())),
        })
    }

    pub async fn new_udp(local_addr: SocketAddr) -> std::io::Result<Self> {
        let socket = socket2::Socket::new(
            Domain::for_address(local_addr),
            Type::DGRAM,
            Some(socket2::Protocol::UDP),
        )?;
        socket.set_reuse_address(true)?;
        socket.set_reuse_port(true)?;
        socket.set_nonblocking(true)?;
        socket.bind(&local_addr.into())?;

        let std_socket: std::net::UdpSocket = socket.into();
        let tokio_socket = tokio::net::UdpSocket::from_std(std_socket)?;
        let real_local_addr = tokio_socket.local_addr()?;
        let socket = Arc::new(tokio_socket);
        let peers = Arc::new(tokio::sync::Mutex::new(HashMap::new()));
        let (new_connection_tx, new_connection_rx) = mpsc::channel(100);
        let server = Self {
            inner: ServerEndpointIoInner::Udp {
                socket: socket.clone(),
                peers: peers.clone(),
                new_connection_rx,
            },
            protocol: Protocol::Udp,
            local_addr: real_local_addr,
            shared: Arc::new(tokio::sync::Mutex::new(Shared::default())),
        };
        tokio::spawn(Self::udp_recv_loop(
            real_local_addr,
            server.shared.clone(),
            socket,
            peers,
            new_connection_tx,
        ));
        Ok(server)
    }

    async fn udp_recv_loop(
        local_addr: SocketAddr,
        shared: Arc<tokio::sync::Mutex<Shared>>,
        socket: Arc<UdpSocket>,
        peers: Arc<tokio::sync::Mutex<HashMap<SocketAddr, mpsc::Sender<Bytes>>>>,
        new_connection_tx: mpsc::Sender<ConnectionIo>,
    ) {
        let mut buf = Vec::with_capacity(1500);
        tracing::debug!("udp recv start, local: {}", local_addr);
        loop {
            buf.clear();
            match socket.recv_buf_from(&mut buf).await {
                Ok((len, addr)) => {
                    let data = Bytes::copy_from_slice(&buf[..len]);
                    let mut peers_guard = peers.lock().await;
                    if let Some(tx) = peers_guard.get(&addr) {
                        if let Err(err) = tx.try_send(data) {
                            tracing::error!(
                                "error sending data to receiver {} -> {}: {}",
                                addr,
                                local_addr,
                                err
                            );
                            peers_guard.remove(&addr);
                        }
                    } else {
                        tracing::debug!(
                            "udp new connection recved, len={}, local={}, remote: {}",
                            len,
                            local_addr,
                            addr
                        );
                        let (tx, rx) = mpsc::channel(1000);
                        peers_guard.insert(addr, tx);
                        let connection = ConnectionIo::new(
                            ConnectionIoInner::Udp {
                                peers: Some(peers.clone()),
                                socket: socket.clone(),
                                bytes_rx: rx,
                                read_buffer: Some(data),
                            },
                            addr,
                            local_addr,
                            Some(shared.clone()),
                        );
                        if let Err(err) = new_connection_tx.send(connection).await {
                            tracing::error!("error sending new connection notification: {}", err);
                            peers_guard.remove(&addr);
                            continue;
                        }
                    }
                }
                Err(err) => {
                    tracing::error!("error receiving udp packet: {}", err);
                    break;
                }
            }
        }
        tracing::debug!("udp recv detached, local: {}", local_addr);
    }

    pub fn protocol(&self) -> Protocol {
        self.protocol
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub async fn accept(&mut self) -> std::io::Result<ConnectionIo> {
        match &mut self.inner {
            ServerEndpointIoInner::Tcp(listener) => {
                let (stream, addr) = listener.accept().await?;
                self.shared.lock().await.number_of_connections += 1;
                Ok(ConnectionIo::new(
                    ConnectionIoInner::Tcp(stream),
                    addr,
                    self.local_addr,
                    Some(self.shared.clone()),
                ))
            }
            ServerEndpointIoInner::Udp {
                new_connection_rx, ..
            } => {
                let rx = new_connection_rx.recv().await;
                match rx {
                    Some(conn) => {
                        self.shared.lock().await.number_of_connections += 1;
                        Ok(conn)
                    }
                    None => Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        "udp server closed",
                    )
                    .into()),
                }
            }
        }
    }
}

impl ConnectionIo {
    pub fn new(
        inner: ConnectionIoInner,
        remote_addr: SocketAddr,
        local_addr: SocketAddr,
        shared: Option<Arc<tokio::sync::Mutex<Shared>>>,
    ) -> Self {
        Self {
            inner,
            remote_addr,
            local_addr,
            bytes_sent: 0,
            bytes_received: 0,
            shared: shared.unwrap_or_else(|| Arc::new(tokio::sync::Mutex::new(Shared::default()))),
            waker: None,
        }
    }

    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub fn bytes_sent(&self) -> usize {
        self.bytes_sent
    }

    pub fn bytes_received(&self) -> usize {
        self.bytes_received
    }
}

impl AsyncRead for ConnectionIo {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        match &mut this.inner {
            ConnectionIoInner::Tcp(stream) => {
                let pinned_stream = std::pin::Pin::new(stream);
                let res = match ready!(pinned_stream.poll_read(cx, buf)) {
                    Ok(()) => {
                        this.bytes_received += buf.filled().len();

                        std::task::Poll::Ready(Ok(()))
                    }
                    other => std::task::Poll::Ready(other),
                };
                if let Some(waker) = this.waker.as_ref() {
                    waker.wake_by_ref();
                }
                res
            }
            ConnectionIoInner::Udp {
                bytes_rx,
                read_buffer,
                ..
            } => {
                if let Some(data) = read_buffer.take() {
                    let len = data.len().min(buf.remaining());
                    buf.put_slice(&data[..len]);
                    this.bytes_received += len;
                    if let Some(waker) = this.waker.as_ref() {
                        waker.wake_by_ref();
                    }
                    return std::task::Poll::Ready(Ok(()));
                }
                let res = match ready!(bytes_rx.poll_recv(cx)) {
                    Some(data) => {
                        let len = data.len().min(buf.remaining());
                        let (a, b) = data.split_at(len);
                        buf.put_slice(a);
                        this.bytes_received += len;
                        read_buffer.replace(Bytes::copy_from_slice(b));
                        std::task::Poll::Ready(Ok(()))
                    }
                    None => std::task::Poll::Ready(Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "udp connection closed",
                    ))),
                };
                if let Some(waker) = this.waker.as_ref() {
                    waker.wake_by_ref();
                }
                res
            }
        }
    }
}

impl AsyncWrite for ConnectionIo {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let this = self.get_mut();
        match &mut this.inner {
            ConnectionIoInner::Tcp(stream) => {
                let pinned_stream = std::pin::Pin::new(stream);
                match pinned_stream.poll_write(cx, buf) {
                    std::task::Poll::Ready(Ok(len)) => {
                        this.bytes_sent += len;
                        std::task::Poll::Ready(Ok(len))
                    }
                    other => other,
                }
            }
            ConnectionIoInner::Udp { socket, .. } => {
                match socket.poll_send_to(cx, buf, this.remote_addr) {
                    std::task::Poll::Ready(Ok(len)) => {
                        this.bytes_sent += len;
                        std::task::Poll::Ready(Ok(len))
                    }
                    other => other,
                }
            }
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        let this = self.get_mut();
        match &mut this.inner {
            ConnectionIoInner::Tcp(stream) => {
                let pinned_stream = std::pin::Pin::new(stream);
                pinned_stream.poll_flush(cx)
            }
            ConnectionIoInner::Udp { .. } => std::task::Poll::Ready(Ok(())),
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        let this = self.get_mut();
        match &mut this.inner {
            ConnectionIoInner::Tcp(stream) => {
                let pinned_stream = std::pin::Pin::new(stream);
                pinned_stream.poll_shutdown(cx)
            }
            ConnectionIoInner::Udp { .. } => std::task::Poll::Ready(Ok(())),
        }
    }

    fn is_write_vectored(&self) -> bool {
        match &self.inner {
            ConnectionIoInner::Tcp(stream) => stream.is_write_vectored(),
            ConnectionIoInner::Udp { .. } => false,
        }
    }

    fn poll_write_vectored(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let this = self.get_mut();
        match &mut this.inner {
            ConnectionIoInner::Tcp(stream) => {
                let pinned_stream = std::pin::Pin::new(stream);
                match pinned_stream.poll_write_vectored(cx, bufs) {
                    std::task::Poll::Ready(Ok(len)) => {
                        this.bytes_sent += len;
                        std::task::Poll::Ready(Ok(len))
                    }
                    other => other,
                }
            }
            ConnectionIoInner::Udp { .. } => {
                unreachable!("poll_write_vectored is not supported for UDP")
            }
        }
    }
}
