use std::{
    collections::HashMap,
    fmt,
    mem::MaybeUninit,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};

use futures::{FutureExt, select};
use iana_formats::{addrress_family::AddressFamilyStatic, protocol_numbers::ProtocolNumberStatic};
use pnet::packet::{
    Packet,
    icmp::{IcmpTypes, destination_unreachable::IcmpCodes},
    icmpv6::Icmpv6Types,
    ip::IpNextHeaderProtocols,
    ipv4::Ipv4Packet,
    ipv6::Ipv6Packet,
    udp::UdpPacket,
};
use socket2::{Domain, Type};
use stun_server::errors::StunSessionResult;
use tokio::{
    net::UdpSocket,
    sync::{RwLock, mpsc, oneshot},
};
use tokio_util::bytes::BytesMut;
use turn_formats::channel_data::ChannelData;

use crate::{
    authenticate_info::AuthenticateInfo,
    channel_binding::{CHANNEL_BINDING_LIFETIME, ChannelBinding},
    errors::{TurnSessionError, TurnSessionResult},
    five_tuple::FiveTuple,
    get_address_family,
    permission::{PERMISSION_LIFETIME, Permission},
};

#[derive(Debug)]
pub enum AllocationCommond {
    CreatePermission {
        peer_ips: Vec<IpAddr>,
        result_tx: oneshot::Sender<TurnSessionResult<()>>,
    },
    CreateChannelBinding {
        channel_number: u16,
        peer_addr: SocketAddr,
        result_tx: oneshot::Sender<TurnSessionResult<()>>,
    },
    Send {
        peer_addr: SocketAddr,
        data: Vec<u8>,
        require_no_fragment: bool,
    },
    ChannelData(ChannelData),
    Refresh {
        lifetime: Option<Duration>,
        address_family: Option<iana_formats::addrress_family::AddressFamily>,
        result_tx: oneshot::Sender<TurnSessionResult<Duration>>,
    },
}

impl fmt::Display for AllocationCommond {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AllocationCommand: ")?;
        match self {
            Self::CreatePermission { peer_ips, .. } => {
                f.write_str("CreatePermission: peer_ips: ")?;
                for ip in peer_ips {
                    write!(f, "{}, ", ip)?;
                }
                Ok(())
            }
            Self::CreateChannelBinding {
                channel_number,
                peer_addr,
                ..
            } => {
                write!(
                    f,
                    "ChannelBinding, channel_number: {}, peer: {}",
                    channel_number, peer_addr
                )
            }
            Self::Send {
                peer_addr,
                data,
                require_no_fragment,
            } => {
                write!(
                    f,
                    "Send, peer: {}, data_len: {}, require_no_fragment: {}",
                    peer_addr,
                    data.len(),
                    require_no_fragment
                )
            }
            Self::ChannelData(data) => {
                write!(
                    f,
                    "ChannelData, channel_number: {}, data_len: {}",
                    data.channel_number,
                    data.application_data.len()
                )
            }
            Self::Refresh {
                lifetime,
                address_family,
                result_tx: _,
            } => {
                write!(f, "Refresh")?;
                if let Some(lifetime) = lifetime {
                    write!(f, ", lifetime: {}s", lifetime.as_secs())?;
                }
                if let Some(family) = address_family {
                    write!(f, ", requested_address_family: {}", family)?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug)]
pub enum AllocationEvent {
    RelayedPacket {
        peer_addr: SocketAddr,
        relay_addr: SocketAddr,
        data: BytesMut,
        channel_number: Option<u16>,
    },
    ICMP {
        source_addr: SocketAddr,
        destination_addr: SocketAddr,
        icmp_type: u8,
        icmp_code: u8,
        mtu: Option<u32>,
    },
}

#[derive(Debug)]
pub struct Allocation {
    fivetuple: FiveTuple,
    relayed_address: SocketAddr,
    relay_endpoint: tokio::net::UdpSocket,
    relay_icmp_endpoint: Option<socket2::Socket>,
    time_to_expiry: Arc<RwLock<Instant>>,
    authenticate_info: AuthenticateInfo,
    permissions: Arc<RwLock<HashMap<IpAddr, Permission>>>,
    channel_bindings: Arc<RwLock<HashMap<u16, ChannelBinding>>>,
}
pub const ALLOCATION_LIFETIME: Duration = Duration::from_mins(10);

pub struct AllocationBuilder {
    protocol: iana_formats::protocol_numbers::Protocol,
    local_addr: Option<SocketAddr>,
    local_ipv4_address: Option<Ipv4Addr>,
    local_ipv6_address: Option<Ipv6Addr>,
    use_icmp: bool,
    remote_addr: Option<SocketAddr>,
    requested_address_family: iana_formats::addrress_family::AddressFamily, // default to ipv4
    lifetime: Duration,
    username: Option<String>,
    realm: Option<String>,
    nonce: Option<String>,
    key_hash: Option<String>,
    require_even_port: bool,
    reservation: Option<UdpSocket>,
}
impl Allocation {
    pub fn new(
        five_tuple: FiveTuple,
        relay_endpoint: tokio::net::UdpSocket,
        relay_icmp_endpoint: Option<socket2::Socket>,
        time_to_expiry: Instant,
        authenticate_info: AuthenticateInfo,
    ) -> Self {
        let relayed_address = relay_endpoint.local_addr().unwrap();
        Self {
            fivetuple: five_tuple,
            relayed_address,
            relay_endpoint,
            relay_icmp_endpoint,
            time_to_expiry: Arc::new(RwLock::new(time_to_expiry)),
            authenticate_info,
            permissions: Default::default(),
            channel_bindings: Default::default(),
        }
    }

    pub fn builder() -> AllocationBuilder {
        AllocationBuilder::new()
    }

    pub fn relayed_transport_address(&self) -> SocketAddr {
        self.relayed_address
    }

    pub async fn relay_udp_endpoint_loop(
        time_to_expiry: Arc<RwLock<Instant>>,
        relayed_address: SocketAddr,
        relay_endpoint: tokio::net::UdpSocket,
        mut outgoing_message: mpsc::Receiver<(Vec<u8>, SocketAddr)>,
        permissions: Arc<RwLock<HashMap<IpAddr, Permission>>>,
        channel_bindings: Arc<RwLock<HashMap<u16, ChannelBinding>>>,
        allocation_event_tx: mpsc::Sender<AllocationEvent>,
    ) -> StunSessionResult<()> {
        loop {
            if *time_to_expiry.read().await < Instant::now() {
                tracing::info!(
                    "relay udp loop exit cuz expired, relay addr: {}",
                    relayed_address
                );
                return Ok(());
            }
            let mut recv_buffer = BytesMut::with_capacity(2048);
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            select! {
                _ = interval.tick().fuse() => {}
                outgoing = outgoing_message.recv().fuse() => {
                    if let Some((message, peer)) = outgoing {
                        tracing::debug!("relay message send to peer: {}, len: {}", peer, message.len());
                        let _ = relay_endpoint.send_to(&message, peer).await.inspect_err(|err| {
                            tracing::error!("error send outgoing relay message to peer: {}, peer: {}, relay addr: {}", err, peer, relayed_address);
                        })?;
                    } else {
                        tracing::warn!("relay outgoing message sender has been closed, relay addr: {}", relayed_address);
                        return Ok(())
                    }
                },
                read_result = relay_endpoint.recv_buf_from(&mut recv_buffer).fuse() => {
                    match read_result {
                        Ok((len, peer)) => {
                            tracing::debug!("relay message recved from peer: {}, relay addr: {}, len: {}", peer, relayed_address, len);

                            if !permissions.read().await.contains_key(&peer.ip()) {
                                tracing::warn!("no permission found for peer {}", peer);
                                continue;
                            }
                            let channel;
                            {
                                let guard = channel_bindings.read().await;
                                channel = guard
                                    .values()
                                    .find(|item| item.peer() == peer.into())
                                    .map(|item| item.channel_number());
                            }
                            let res = allocation_event_tx
                                .send(AllocationEvent::RelayedPacket {
                                    peer_addr: peer.into(),
                                    relay_addr: relayed_address.into(),
                                    data: recv_buffer,
                                    channel_number: channel
                                })
                                .await
                                .map_err(|err| {
                                    std::io::Error::new(std::io::ErrorKind::BrokenPipe,
                                        format!("send incoming relay message to allocation event failed: {}, the allocation must have been ended. relay addr: {}",
                                        err, relayed_address)
                                    )
                                });
                            if let Err(err) = res {
                                tracing::error!("{}", err);
                            }
                        },
                        Err(err) => {
                            tracing::error!("error recving relay data from peer: {}, relay addr: {}", err, relayed_address);
                            return Err(err.into());
                        }
                    }
                }
            }
        }
    }

    fn setup_relay(
        relay_time_to_expiry: Arc<RwLock<Instant>>,
        relay_endpoint: tokio::net::UdpSocket,
        relay_icmp_endpoint: Option<socket2::Socket>,
        relayed_address: SocketAddr,
        outgoing_message: mpsc::Receiver<(IpAddr, Vec<u8>, SocketAddr)>,
        allocation_event_tx: mpsc::Sender<AllocationEvent>,
        permissions: Arc<RwLock<HashMap<IpAddr, Permission>>>,
        channel_bindings: Arc<RwLock<HashMap<u16, ChannelBinding>>>,
    ) -> TurnSessionResult<()> {
        let (relay_outgoing_message_tx, relay_outgoing_message_rx) = mpsc::channel(100);
        let mut relay_handles: HashMap<IpAddr, mpsc::Sender<(Vec<u8>, SocketAddr)>> =
            HashMap::new();
        relay_handles.insert(relayed_address.ip(), relay_outgoing_message_tx);

        let permission_for_relay_udp = permissions.clone();
        let channel_binding_for_relay_udp = channel_bindings.clone();
        let event_tx_for_relay_udp = allocation_event_tx.clone();
        let relay_time_to_expiry_for_udp = relay_time_to_expiry.clone();
        tokio::spawn(async move {
            let res = Self::relay_udp_endpoint_loop(
                relay_time_to_expiry_for_udp,
                relayed_address,
                relay_endpoint,
                relay_outgoing_message_rx,
                permission_for_relay_udp,
                channel_binding_for_relay_udp,
                event_tx_for_relay_udp,
            )
            .await;
            tracing::info!(
                "relay recving loop stopped, res: {:?}, relay addr: {}",
                res,
                relayed_address
            );
        });
        if let Some(icmp_endpoint) = relay_icmp_endpoint {
            let permission_for_relay_icmp = permissions.clone();
            let allocation_event_tx_for_icmp = allocation_event_tx.clone();
            let relay_time_to_expiry_for_icmp = relay_time_to_expiry.clone();
            tokio::spawn(async move {
                let res = Self::relay_icmp_endpoint_loop(
                    relay_time_to_expiry_for_icmp,
                    relayed_address,
                    icmp_endpoint,
                    permission_for_relay_icmp,
                    allocation_event_tx_for_icmp,
                )
                .await;
                tracing::info!(
                    "relay recving icmp loop stopped, res: {:?}, relay addr: {}",
                    res,
                    relayed_address
                );
            });
        }

        tokio::spawn(async move {
            let res =
                Self::process_relay_outgoing(outgoing_message, relay_handles, permissions).await;
            tracing::info!("relay message sending loop stop, res: {:?}", res);
        });
        Ok(())
    }

    async fn relay_icmp_endpoint_loop(
        time_to_expiry: Arc<RwLock<Instant>>,
        relayed_address: SocketAddr,
        relay_icmp_endpoint: socket2::Socket,
        permissions: Arc<RwLock<HashMap<IpAddr, Permission>>>,
        allocation_event_tx: mpsc::Sender<AllocationEvent>,
    ) -> TurnSessionResult<()> {
        let mut buffer = vec![MaybeUninit::uninit(); 1500];
        let is_ipv6 = relayed_address.is_ipv6();
        let fd = tokio::io::unix::AsyncFd::new(relay_icmp_endpoint)?;
        loop {
            if *time_to_expiry.read().await < Instant::now() {
                tracing::info!(
                    "relay icmp loop exit cuz expired, relay addr: {}",
                    relayed_address
                );
                return Ok(());
            }
            let gaurd = tokio::time::timeout(Duration::from_secs(10), fd.readable()).await;
            match gaurd {
                Err(_) => {}
                Ok(Err(err)) => return Err(err.into()),
                Ok(Ok(mut gaurd)) => {
                    match gaurd.try_io(|inner| {
                        let sock = inner.get_ref();
                        sock.recv_from(&mut buffer)
                    }) {
                        Ok(Ok((len, remote))) => {
                            tracing::debug!(
                                "got icmp packet from: {:?}, len: {}, local: {}",
                                remote,
                                len,
                                relayed_address
                            );
                            let payload = unsafe {
                                std::slice::from_raw_parts(buffer.as_ptr() as *mut u8, len)
                            };
                            if let Some((
                                source_addr,
                                destination_addr,
                                icmp_type,
                                icmp_code,
                                mtu,
                            )) = Self::handle_icmp_packet(is_ipv6, payload)?
                            {
                                if source_addr != relayed_address {
                                    tracing::debug!(
                                        "icmp source addr: {} not match relay addr: {}",
                                        source_addr,
                                        relayed_address
                                    );
                                    continue;
                                }
                                if !permissions
                                    .read()
                                    .await
                                    .contains_key(&destination_addr.ip())
                                {
                                    tracing::debug!(
                                        "no permission found for icmp dest addr: {}",
                                        destination_addr
                                    );
                                    continue;
                                }
                                let res = allocation_event_tx
                                .send(AllocationEvent::ICMP {
                                    source_addr,
                                    destination_addr,
                                    icmp_type,
                                    icmp_code,
                                    mtu,
                                })
                                .await
                                .map_err(|err| {
                                    std::io::Error::new(std::io::ErrorKind::BrokenPipe,
                                        format!("send incoming relay message to allocation event failed: {}, the allocation must have been ended. relay addr: {}",
                                        err, relayed_address)
                                    )
                                });
                                if let Err(err) = res {
                                    tracing::error!("{}", err);
                                }
                            }
                        }
                        Ok(Err(err)) if err.kind() == std::io::ErrorKind::WouldBlock => continue,
                        Ok(Err(err)) => return Err(err.into()),
                        Err(_err) => continue,
                    }
                }
            }
        }
    }

    fn handle_icmp_packet(
        is_ipv6: bool,
        buffer: &[u8],
    ) -> TurnSessionResult<Option<(SocketAddr, SocketAddr, u8, u8, Option<u32>)>> {
        if !is_ipv6 {
            return Self::handle_icmpv4_packet(buffer);
        }
        Self::handle_icmpv6_packet(buffer)
    }

    fn handle_icmpv6_packet(
        buffer: &[u8],
    ) -> TurnSessionResult<Option<(SocketAddr, SocketAddr, u8, u8, Option<u32>)>> {
        let packet = pnet::packet::icmpv6::Icmpv6Packet::new(buffer);
        if packet.is_none() {
            tracing::debug!("no icmpv6 packet parsed");
            return Ok(None);
        }
        let packet = packet.unwrap();
        tracing::debug!("got ICMPv6 packet: {:?}", packet);
        let icmp_type = packet.get_icmpv6_type();
        let icmp_code = packet.get_icmpv6_code();
        if icmp_type != Icmpv6Types::DestinationUnreachable
            && icmp_type != Icmpv6Types::PacketTooBig
            && icmp_type != Icmpv6Types::TimeExceeded
        {
            tracing::debug!("icmp packet type not interested: {:?}", icmp_type);
            return Ok(None);
        }
        let mut mtu = None;
        if icmp_type == Icmpv6Types::PacketTooBig {
            mtu = Some(u32::from_be_bytes([
                buffer[4], buffer[5], buffer[6], buffer[7],
            ]));
        }
        let inner_ip = Ipv6Packet::new(packet.payload());
        if inner_ip.is_none() {
            tracing::debug!("no ipv6 packet parsed");
            return Ok(None);
        }
        let inner_ip = inner_ip.unwrap();
        let source_ip = inner_ip.get_source();
        let dest_ip = inner_ip.get_destination();
        let next_level = inner_ip.get_next_header();
        if next_level != IpNextHeaderProtocols::Udp {
            tracing::debug!("ipv6 packet next level protocol not udp: {}", next_level);
            return Ok(None);
        }
        if let Some((source, dest)) = Self::handle_icmp_udp_packet(inner_ip.payload())? {
            return Ok(Some((
                SocketAddr::new(IpAddr::V6(source_ip), source),
                SocketAddr::new(IpAddr::V6(dest_ip), dest),
                icmp_type.0,
                icmp_code.0,
                mtu,
            )));
        }
        Ok(None)
    }

    fn handle_icmp_udp_packet(buffer: &[u8]) -> TurnSessionResult<Option<(u16, u16)>> {
        let udp = UdpPacket::new(buffer);
        if udp.is_none() {
            tracing::debug!("no udp packet parsed");
            return Ok(None);
        }
        let udp = udp.unwrap();
        let source = udp.get_source();
        let dest = udp.get_destination();
        Ok(Some((source, dest)))
    }

    fn handle_icmpv4_packet(
        buffer: &[u8],
    ) -> TurnSessionResult<Option<(SocketAddr, SocketAddr, u8, u8, Option<u32>)>> {
        let packet = pnet::packet::icmp::IcmpPacket::new(buffer);
        if packet.is_none() {
            tracing::debug!("no icmp packet parsed");
            return Ok(None);
        }
        let packet = packet.unwrap();
        tracing::debug!("got ICMP packet: {:?}", packet);
        let icmp_type = packet.get_icmp_type();
        let icmp_code = packet.get_icmp_code();
        if icmp_type != IcmpTypes::DestinationUnreachable && icmp_type != IcmpTypes::TimeExceeded {
            tracing::debug!("icmp packet type not interested: {:?}", icmp_type);
            return Ok(None);
        }
        let mut mtu = None;
        if icmp_type == IcmpTypes::DestinationUnreachable
            && icmp_code == IcmpCodes::FragmentationRequiredAndDFFlagSet
        {
            mtu = Some(u16::from_be_bytes([buffer[6], buffer[7]]) as u32);
        }
        let inner_ip = Ipv4Packet::new(packet.payload());
        if inner_ip.is_none() {
            tracing::debug!("no ipv4 packet parsed");
            return Ok(None);
        }
        let inner_ip = inner_ip.unwrap();
        let source_ip = inner_ip.get_source();
        let dest_ip = inner_ip.get_destination();
        let next_level = inner_ip.get_next_level_protocol();
        if next_level != IpNextHeaderProtocols::Udp {
            tracing::debug!("ipv4 packet next level protocol not udp: {}", next_level);
            return Ok(None);
        }
        if let Some((source, dest)) = Self::handle_icmp_udp_packet(inner_ip.payload())? {
            return Ok(Some((
                SocketAddr::new(IpAddr::V4(source_ip), source),
                SocketAddr::new(IpAddr::V4(dest_ip), dest),
                icmp_type.0,
                icmp_code.0,
                mtu,
            )));
        }
        Ok(None)
    }

    async fn process_relay_outgoing(
        mut outgoing_message: mpsc::Receiver<(IpAddr, Vec<u8>, SocketAddr)>,
        relay_handles: HashMap<IpAddr, mpsc::Sender<(Vec<u8>, SocketAddr)>>,
        permissions: Arc<RwLock<HashMap<IpAddr, Permission>>>,
    ) -> TurnSessionResult<()> {
        while let Some((local, message, peer)) = outgoing_message.recv().await {
            tracing::debug!(
                "process outgoing, peer: {}, local: {}, message len: {}",
                peer,
                local,
                message.len()
            );
            if !relay_handles.contains_key(&local) {
                tracing::warn!("address {} is not opened for relay", local);
                continue;
            }

            if !permissions.read().await.contains_key(&peer.ip()) {
                tracing::warn!("no permission found for peer {}", peer);
                continue;
            }
            let res = relay_handles
                .get(&local)
                .unwrap()
                .send((message, peer))
                .await
                .map_err(|err| {
                    std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        format!(
                            "send outgoing relay message to relay socket failed: {}, peer: {}, relay addr: {}",
                            err, peer, local
                        ),
                    )
                });
            if let Err(err) = res {
                tracing::error!("{}", err);
                return Err(err.into());
            }
        }
        Ok(())
    }

    pub async fn run(
        self,
        mut allocation_command_rx: mpsc::Receiver<AllocationCommond>,
        allocation_event_tx: mpsc::Sender<AllocationEvent>,
    ) -> TurnSessionResult<()> {
        let stop = Arc::new(AtomicBool::new(false));
        let (outgoing_message_tx, outgoing_message_rx) = mpsc::channel(100);
        let time_expiry_for_relay = self.time_to_expiry.clone();
        Self::setup_relay(
            time_expiry_for_relay,
            self.relay_endpoint,
            self.relay_icmp_endpoint,
            self.relayed_address,
            outgoing_message_rx,
            allocation_event_tx.clone(),
            self.permissions.clone(),
            self.channel_bindings.clone(),
        )?;
        let permissions = self.permissions.clone();
        let stop_for_permission = stop.clone();
        tokio::spawn(async move {
            loop {
                if stop_for_permission.load(std::sync::atomic::Ordering::Acquire) {
                    return;
                }
                permissions
                    .write()
                    .await
                    .retain(|_, value| !value.expired());
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });
        let channl_bindings_for_refresh = self.channel_bindings.clone();
        let stop_for_channel_binding = stop.clone();
        tokio::spawn(async move {
            loop {
                if stop_for_channel_binding.load(std::sync::atomic::Ordering::Acquire) {
                    return;
                }
                channl_bindings_for_refresh
                    .write()
                    .await
                    .retain(|_, value| !value.expired());
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });

        loop {
            let now = Instant::now();
            if *self.time_to_expiry.read().await < now {
                tracing::info!("allocation is about to exit cuz expired");
                break;
            }
            match tokio::time::timeout(Duration::from_secs(10), allocation_command_rx.recv()).await
            {
                Ok(Some(command)) => {
                    tracing::debug!("allocation command: {}", command);
                    match command {
                        AllocationCommond::CreatePermission {
                            peer_ips,
                            result_tx,
                        } => {
                            Self::process_create_permission(
                                &self.permissions,
                                peer_ips,
                                result_tx,
                                &self.relayed_address,
                            )
                            .await?;
                        }
                        AllocationCommond::CreateChannelBinding {
                            channel_number,
                            peer_addr,
                            result_tx,
                        } => {
                            Self::process_create_channel_binding(
                                &self.channel_bindings,
                                &self.permissions,
                                channel_number,
                                peer_addr,
                                result_tx,
                            )
                            .await?;
                        }
                        AllocationCommond::Send {
                            peer_addr,
                            data,
                            require_no_fragment,
                        } => {
                            Self::process_send(
                                peer_addr,
                                data,
                                require_no_fragment,
                                &self.relayed_address,
                                &outgoing_message_tx,
                            )
                            .await?;
                        }
                        AllocationCommond::ChannelData(data) => {
                            Self::process_channel_data(
                                &self.channel_bindings,
                                data,
                                &self.relayed_address,
                                &outgoing_message_tx,
                            )
                            .await?;
                        }
                        AllocationCommond::Refresh {
                            lifetime,
                            address_family: _,
                            result_tx,
                        } => {
                            Self::process_refresh(&self.time_to_expiry, lifetime, result_tx)
                                .await?;
                        }
                    }
                }
                Ok(None) => {
                    tracing::info!(
                        "allocation is about to exit cuz command channel has been closed"
                    );
                    break;
                }
                Err(_) => {}
            }
        }
        stop.store(true, std::sync::atomic::Ordering::Release);
        Ok(())
    }

    async fn process_refresh(
        time_to_expiry: &Arc<RwLock<Instant>>,
        lifetime: Option<Duration>,
        result_tx: oneshot::Sender<TurnSessionResult<Duration>>,
    ) -> TurnSessionResult<()> {
        let lifetime = lifetime.unwrap_or(ALLOCATION_LIFETIME);
        let lifetime = if lifetime.is_zero() {
            lifetime
        } else {
            lifetime.max(ALLOCATION_LIFETIME)
        };
        let instant = Instant::now().checked_add(lifetime).unwrap();
        *time_to_expiry.write().await = instant;
        result_tx.send(Ok(lifetime)).map_err(|err| {
            std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!(
                    "send create channel binding success back to session failed: {:?}",
                    err
                ),
            )
        })?;
        Ok(())
    }

    async fn process_channel_data(
        channel_bindings: &Arc<RwLock<HashMap<u16, ChannelBinding>>>,
        data: ChannelData,
        local_addr: &SocketAddr,
        outgoing_message_tx: &mpsc::Sender<(IpAddr, Vec<u8>, SocketAddr)>,
    ) -> TurnSessionResult<()> {
        let binded_addr = channel_bindings
            .read()
            .await
            .get(&data.channel_number)
            .map(|item| item.peer());
        if binded_addr.is_none() {
            tracing::warn!(
                "got channel data but not associated channel binding found, channel number: {}",
                data.channel_number
            );
            return Ok(());
        }
        let binded_addr = binded_addr.unwrap();
        let local_addrss_family = iana_formats::addrress_family::for_address(local_addr);
        if get_address_family(binded_addr.ip()) != local_addrss_family {
            tracing::warn!(
                "unable to relay channel data from channel: {}, cuz the binded peer address: {} is of unsupported ip family",
                data.channel_number,
                binded_addr
            );
            return Ok(());
        }
        outgoing_message_tx
            .send((local_addr.ip(), data.application_data, binded_addr))
            .await
            .map_err(|err| {
                std::io::Error::new(std::io::ErrorKind::BrokenPipe,
                    format!("send outgoing message from channel data to relay failed: {}, peer: {}, relay addr: {}", err, binded_addr, local_addr))
            })?;
        Ok(())
    }

    async fn process_send(
        peer_addr: SocketAddr,
        data: Vec<u8>,
        require_no_fragment: bool,
        local_addr: &SocketAddr,
        outgoing_message_tx: &mpsc::Sender<(IpAddr, Vec<u8>, SocketAddr)>,
    ) -> TurnSessionResult<()> {
        let local_address_family = iana_formats::addrress_family::for_address(local_addr);
        if get_address_family(peer_addr.ip()) != local_address_family {
            tracing::warn!("got send data with unsupported peer addr: {}", peer_addr);
            return Ok(());
        }

        outgoing_message_tx
            .send((local_addr.ip(), data, peer_addr))
            .await
            .map_err(|err| {
                std::io::Error::new(std::io::ErrorKind::BrokenPipe,
                    format!("send outgoing message from send indication to relay failed: {}, peer: {}, relay addr: {}", err, peer_addr, local_addr))
            })?;
        Ok(())
    }

    async fn process_create_channel_binding(
        channel_bindings: &Arc<RwLock<HashMap<u16, ChannelBinding>>>,
        permissions: &Arc<RwLock<HashMap<IpAddr, Permission>>>,
        channel_number: u16,
        peer_addr: SocketAddr,
        result_tx: oneshot::Sender<TurnSessionResult<()>>,
    ) -> TurnSessionResult<()> {
        let mut channel_check = true;
        {
            let guard = channel_bindings.read().await;
            if let Some(entry) = guard.get(&channel_number)
                && entry.peer() != peer_addr
            {
                channel_check = false;
            }
            if let Some(entry) = guard.values().find(|entry| entry.peer() == peer_addr)
                && entry.channel_number() != channel_number
            {
                channel_check = false;
            }
        };
        if !channel_check {
            result_tx
                .send(Err(TurnSessionError::ChannelNotMatch {
                    channel_number,
                    peer: peer_addr,
                }))
                .map_err(|err| {
                    std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        format!(
                            "send create channel binding failed back to session failed: {:?}",
                            err
                        ),
                    )
                })?;
            return Ok(());
        }
        let time_to_expiry = Instant::now()
            .checked_add(CHANNEL_BINDING_LIFETIME)
            .unwrap();
        channel_bindings
            .write()
            .await
            .entry(channel_number)
            .or_insert(ChannelBinding::new(
                channel_number,
                peer_addr,
                time_to_expiry,
            ))
            .refresh();
        permissions
            .write()
            .await
            .entry(peer_addr.ip())
            .or_insert(Permission::new(
                peer_addr.ip(),
                Instant::now().checked_add(PERMISSION_LIFETIME).unwrap(),
            ))
            .refresh();
        result_tx.send(Ok(())).map_err(|err| {
            std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!(
                    "send create channel binding success back to session failed: {:?}",
                    err
                ),
            )
        })?;
        Ok(())
    }

    async fn process_create_permission(
        permissions: &Arc<RwLock<HashMap<IpAddr, Permission>>>,
        peer_ips: Vec<IpAddr>,
        result_tx: oneshot::Sender<TurnSessionResult<()>>,
        local_addr: &SocketAddr,
    ) -> TurnSessionResult<()> {
        let time_to_expiry = Instant::now().checked_add(PERMISSION_LIFETIME).unwrap();
        let mut guard = permissions.write().await;
        let mut result = Ok(());
        let local_ip_family = iana_formats::addrress_family::for_address(local_addr);
        for ip in peer_ips {
            let ip_family: iana_formats::addrress_family::AddressFamily = get_address_family(ip);
            if ip_family != local_ip_family {
                tracing::error!(
                    "found unsupported ip family in create permission request, ip: {}, family: {}",
                    ip,
                    ip_family
                );
                result = Err(TurnSessionError::PeerAddressFamilyNotMatch(ip_family))
            }
            guard
                .entry(ip)
                .or_insert(Permission::new(ip, time_to_expiry))
                .refresh();
        }
        result_tx.send(result).map_err(|err| {
            std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!(
                    "send create permission success back to session failed: {:?}",
                    err
                ),
            )
        })?;
        Ok(())
    }
}

impl Default for AllocationBuilder {
    fn default() -> Self {
        Self {
            local_addr: None,
            remote_addr: None,
            requested_address_family: iana_formats::addrress_family::IPv4::ADDRESS_FAMILY,
            lifetime: Duration::from_mins(10),
            username: None,
            realm: None,
            nonce: None,
            key_hash: None,
            protocol: iana_formats::protocol_numbers::UDP::PROTOCOL,
            local_ipv4_address: None,
            local_ipv6_address: None,
            use_icmp: false,
            require_even_port: false,
            reservation: None,
        }
    }
}

impl AllocationBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn local_addr(mut self, addr: SocketAddr) -> Self {
        self.local_addr = Some(addr);
        self
    }

    pub fn remote_addr(mut self, addr: SocketAddr) -> Self {
        self.remote_addr = Some(addr);
        self
    }

    pub fn local_ipv4_addr(mut self, addr: Ipv4Addr) -> Self {
        self.local_ipv4_address = Some(addr);
        self
    }

    pub fn local_ipv6_addr(mut self, addr: Option<Ipv6Addr>) -> Self {
        self.local_ipv6_address = addr;
        self
    }

    pub fn use_icmp(mut self, use_icmp: bool) -> Self {
        self.use_icmp = use_icmp;
        self
    }

    pub fn protocol(
        mut self,
        protocol: iana_formats::protocol_numbers::Protocol,
    ) -> TurnSessionResult<Self> {
        if protocol != iana_formats::protocol_numbers::UDP::PROTOCOL {
            return Err(crate::errors::TurnSessionError::UnsupportedProtocol(
                protocol,
            ));
        }
        self.protocol = protocol;
        Ok(self)
    }

    pub fn requested_address_family(
        mut self,
        family: iana_formats::addrress_family::AddressFamily,
    ) -> TurnSessionResult<Self> {
        if ![
            iana_formats::addrress_family::IPv4::ADDRESS_FAMILY,
            iana_formats::addrress_family::IPv6::ADDRESS_FAMILY,
        ]
        .contains(&family)
        {
            return Err(crate::errors::TurnSessionError::UnsupportedAddressFamily(
                family,
            ));
        }

        self.requested_address_family = family;
        Ok(self)
    }

    pub fn relay_address_family(
        self,
        family: Option<iana_formats::addrress_family::AddressFamily>,
    ) -> TurnSessionResult<Self> {
        if let Some(family) = family {
            return self.requested_address_family(family);
        }
        return Ok(self);
    }

    pub fn lifetime(mut self, duration: Duration) -> Self {
        self.lifetime = duration;
        self
    }

    pub fn username<S: Into<String>>(mut self, username: S) -> Self {
        self.username = Some(username.into());
        self
    }

    pub fn realm<S: Into<String>>(mut self, realm: S) -> Self {
        self.realm = Some(realm.into());
        self
    }

    pub fn nonce<S: Into<String>>(mut self, nonce: S) -> Self {
        self.nonce = Some(nonce.into());
        self
    }

    pub fn key_hash<S: Into<String>>(mut self, hash: S) -> Self {
        self.key_hash = Some(hash.into());
        self
    }

    pub fn require_even_port(mut self, require_even_port: bool) -> Self {
        self.require_even_port = require_even_port;
        self
    }

    pub fn with_reservation(mut self, reservation: Option<UdpSocket>) -> Self {
        self.reservation = reservation;
        self
    }

    fn make_endpoint(address: SocketAddr, require_even_port: bool) -> TurnSessionResult<UdpSocket> {
        let mut count = 0;
        let mut socket;
        loop {
            socket = socket2::Socket::new(
                Domain::for_address(address),
                Type::DGRAM,
                Some(socket2::Protocol::UDP),
            )?;
            socket.set_reuse_address(true)?;
            socket.set_reuse_port(true)?;
            socket.set_nonblocking(true)?;
            socket.bind(&address.into())?;
            if !require_even_port {
                break;
            }
            let port = socket.local_addr()?.as_socket().unwrap().port();
            if port.is_multiple_of(2) {
                break;
            }
            count += 1;
            if count == 10 {
                return Err(TurnSessionError::NoEvenPortAvaliable);
            }
        }
        let std_socket: std::net::UdpSocket = socket.into();
        let udp_socket = UdpSocket::from_std(std_socket)?;

        Ok(udp_socket)
    }

    fn make_icmp_endpoint(address: SocketAddr) -> TurnSessionResult<socket2::Socket> {
        let ip_protocol = match address {
            SocketAddr::V4(_) => socket2::Protocol::ICMPV4,
            SocketAddr::V6(_) => socket2::Protocol::ICMPV6,
        };
        let icmp_socket =
            socket2::Socket::new(Domain::for_address(address), Type::RAW, Some(ip_protocol))?;
        icmp_socket.set_nonblocking(true)?;
        icmp_socket.bind(&address.into())?;
        Ok(icmp_socket)
    }

    async fn make_ipv4_endpoint(
        &self,
    ) -> TurnSessionResult<Option<(UdpSocket, Option<socket2::Socket>)>> {
        if self.local_ipv4_address.is_none() {
            return Ok(None);
        }

        let address = SocketAddr::new(IpAddr::V4(self.local_ipv4_address.unwrap()), 0);
        let socket = Self::make_endpoint(address, self.require_even_port)?;
        let icmp_socket = if self.use_icmp {
            Some(Self::make_icmp_endpoint(address)?)
        } else {
            None
        };
        Ok(Some((socket, icmp_socket)))
    }

    async fn make_ipv6_endpoint(
        &self,
    ) -> TurnSessionResult<Option<(UdpSocket, Option<socket2::Socket>)>> {
        if self.local_ipv6_address.is_none() {
            return Ok(None);
        }
        let address = SocketAddr::new(IpAddr::V6(self.local_ipv6_address.unwrap()), 0);
        let socket = Self::make_endpoint(address, self.require_even_port)?;
        let icmp_socket = if self.use_icmp {
            Some(Self::make_icmp_endpoint(address)?)
        } else {
            None
        };
        Ok(Some((socket, icmp_socket)))
    }

    async fn make_requested_endpoint(
        &self,
    ) -> TurnSessionResult<(UdpSocket, Option<socket2::Socket>)> {
        if self.requested_address_family == iana_formats::addrress_family::IPv4::ADDRESS_FAMILY {
            return self.make_ipv4_endpoint().await?.ok_or(
                crate::errors::TurnSessionError::UnsupportedAddressFamily(
                    iana_formats::addrress_family::IPv4::ADDRESS_FAMILY,
                ),
            );
        }
        if self.requested_address_family == iana_formats::addrress_family::IPv6::ADDRESS_FAMILY {
            return self.make_ipv6_endpoint().await?.ok_or(
                crate::errors::TurnSessionError::UnsupportedAddressFamily(
                    iana_formats::addrress_family::IPv6::ADDRESS_FAMILY,
                ),
            );
        }
        unreachable!("unsupported ip family: {}", self.requested_address_family);
    }

    pub async fn build(mut self) -> TurnSessionResult<Allocation> {
        if self.local_addr.is_none() {
            return Err(crate::errors::TurnSessionError::BuildAllocationError(
                "local addr must be set before building".to_owned(),
            ));
        }
        if self.remote_addr.is_none() {
            return Err(crate::errors::TurnSessionError::BuildAllocationError(
                "remote addr must be set before building".to_owned(),
            ));
        }

        let five_tuple = FiveTuple::new(
            self.protocol,
            self.local_addr.unwrap().into(),
            self.remote_addr.unwrap().into(),
        );

        let (udp_endpoint, icmp_endpoint) = if let Some(socket) = self.reservation.take() {
            let relay_address = socket.local_addr()?;
            self.requested_address_family =
                iana_formats::addrress_family::for_address(&relay_address);
            let icmp_endpoint = if self.use_icmp {
                Some(Self::make_icmp_endpoint(relay_address)?)
            } else {
                None
            };
            (socket, icmp_endpoint)
        } else {
            self.make_requested_endpoint().await?
        };

        let time_to_expiry = std::time::Instant::now()
            .checked_add(self.lifetime)
            .unwrap();
        let auth_info = if self.username.is_none()
            && self.realm.is_none()
            && self.nonce.is_none()
            && self.key_hash.is_none()
        {
            Default::default()
        } else {
            AuthenticateInfo {
                username: self.username.unwrap_or_default(),
                realm: self.realm.unwrap_or_default(),
                nonce: self.nonce.unwrap_or_default(),
                key_hash: self.key_hash.unwrap_or_default(),
            }
        };

        Ok(Allocation::new(
            five_tuple,
            udp_endpoint,
            icmp_endpoint,
            time_to_expiry,
            auth_info,
        ))
    }
}
