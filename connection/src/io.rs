use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpListener, TcpSocket, UdpSocket},
    sync::mpsc,
};
use tokio_util::bytes::Bytes;

use crate::errors::ConnResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Tcp,
    Udp,
    Pipe,
}

#[derive(Debug)]
pub struct ClientEndpointIo {
    protocol: Protocol,
    local_addr: SocketAddr,
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
}

impl Drop for ConnectionIo {
    fn drop(&mut self) {
        let shared = self.shared.clone();
        tokio::spawn(async move {
            let mut shared = shared.lock().await;
            if shared.number_of_connections > 0 {
                shared.number_of_connections -= 1;
            }
        });
    }
}

impl ClientEndpointIo {
    pub fn new_tcp(local_addr: SocketAddr) -> ConnResult<Self> {
        Ok(Self {
            protocol: Protocol::Tcp,
            local_addr,
        })
    }

    pub fn new_udp(addr: SocketAddr) -> ConnResult<Self> {
        Ok(Self {
            protocol: Protocol::Udp,
            local_addr: addr,
        })
    }

    pub fn protocol(&self) -> Protocol {
        self.protocol
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub async fn connect(self, remote: SocketAddr) -> ConnResult<ConnectionIo> {
        match self.protocol {
            Protocol::Tcp => {
                let tcp = match self.local_addr.ip() {
                    std::net::IpAddr::V4(_) => TcpSocket::new_v4(),
                    std::net::IpAddr::V6(_) => TcpSocket::new_v6(),
                }?;
                tcp.bind(self.local_addr)?;
                let stream = tcp.connect(remote).await?;
                Ok(ConnectionIo::new(
                    ConnectionIoInner::Tcp(stream),
                    remote,
                    self.local_addr,
                    None,
                ))
            }
            Protocol::Udp => {
                let udp = UdpSocket::bind(self.local_addr).await?;
                udp.connect(remote).await?;
                let (tx, rx) = mpsc::channel(100);
                let socket = Arc::new(udp);
                let client = ConnectionIo::new(
                    ConnectionIoInner::Udp {
                        socket: socket.clone(),
                        bytes_rx: rx,
                        read_buffer: None,
                    },
                    remote,
                    self.local_addr,
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
                                        self.local_addr,
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
            _ => unreachable!("unsupported protocol"),
        }
    }
}

impl ServerEndpointIo {
    pub async fn new_tcp(local_addr: SocketAddr) -> std::io::Result<Self> {
        let listener = TcpListener::bind(local_addr).await?;
        Ok(Self {
            inner: ServerEndpointIoInner::Tcp(listener),
            protocol: Protocol::Tcp,
            local_addr,
            shared: Arc::new(tokio::sync::Mutex::new(Shared::default())),
        })
    }

    pub async fn new_udp(local_addr: SocketAddr) -> std::io::Result<Self> {
        let socket = Arc::new(UdpSocket::bind(local_addr).await?);
        let peers = Arc::new(tokio::sync::Mutex::new(HashMap::new()));
        let (new_connection_tx, new_connection_rx) = mpsc::channel(100);
        let server = Self {
            inner: ServerEndpointIoInner::Udp {
                socket: socket.clone(),
                peers: peers.clone(),
                new_connection_rx,
            },
            protocol: Protocol::Udp,
            local_addr,
            shared: Arc::new(tokio::sync::Mutex::new(Shared::default())),
        };
        tokio::spawn(Self::udp_recv_loop(
            local_addr,
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
        let mut buf = vec![0u8; 2048];
        loop {
            match socket.recv_from(&mut buf).await {
                Ok((len, addr)) => {
                    let data = Bytes::copy_from_slice(&buf[..len]);
                    let mut peers = peers.lock().await;
                    if let Some(tx) = peers.get(&addr) {
                        if let Err(err) = tx.send(data).await {
                            tracing::error!(
                                "error sending data to receiver {} -> {}: {}",
                                addr,
                                local_addr,
                                err
                            );
                            peers.remove(&addr);
                        }
                    } else {
                        let (tx, rx) = mpsc::channel(100);
                        peers.insert(addr, tx.clone());
                        let connection = ConnectionIo::new(
                            ConnectionIoInner::Udp {
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
                            peers.remove(&addr);
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
    }

    pub fn protocol(&self) -> Protocol {
        self.protocol
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub async fn accept(&mut self) -> ConnResult<ConnectionIo> {
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
                match pinned_stream.poll_read(cx, buf) {
                    std::task::Poll::Ready(Ok(())) => {
                        this.bytes_received += buf.filled().len();
                        std::task::Poll::Ready(Ok(()))
                    }
                    other => other,
                }
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
                    return std::task::Poll::Ready(Ok(()));
                }
                match bytes_rx.poll_recv(cx) {
                    std::task::Poll::Ready(Some(data)) => {
                        let len = data.len().min(buf.remaining());
                        let (a, b) = data.split_at(len);
                        buf.put_slice(a);
                        this.bytes_received += len;
                        read_buffer.replace(Bytes::copy_from_slice(b));
                        std::task::Poll::Ready(Ok(()))
                    }
                    std::task::Poll::Ready(None) => {
                        std::task::Poll::Ready(Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "udp connection closed",
                        )))
                    }
                    std::task::Poll::Pending => std::task::Poll::Pending,
                }
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
            ConnectionIoInner::Udp { socket, .. } => match socket.poll_send(cx, buf) {
                std::task::Poll::Ready(Ok(len)) => {
                    this.bytes_sent += len;
                    std::task::Poll::Ready(Ok(len))
                }
                other => other,
            },
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
