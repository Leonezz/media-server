use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    allocation::{ALLOCATION_LIFETIME, Allocation, AllocationCommond, AllocationEvent},
    errors::{TurnSessionError, TurnSessionResult},
    five_tuple::FiveTuple,
};
use iana_formats::{addrress_family::AddressFamilyStatic, protocol_numbers::Protocol};
use socket2::{Domain, Type};
use stun_formats::{
    attributes::AttributeExtStatic,
    header::{MessageClass, TransactionId},
    methods::MethodExtStatic,
};
use tokio::{
    net::UdpSocket,
    sync::{RwLock, mpsc, oneshot},
};
use turn_formats::{attributes, channel_data::ChannelData, message::Message, methods};

const RESERVATION_LIFETIME: Duration = Duration::from_mins(1);

#[derive(Debug)]
pub struct Session {
    local_ipv4_address: Ipv4Addr,
    local_ipv6_address: Option<Ipv6Addr>,
    use_icmp: bool,

    fivetuple: FiveTuple,
    message_tx: mpsc::Sender<(Message, Option<oneshot::Sender<()>>)>,
    message_rx: mpsc::Receiver<Message>,

    allocation_set: bool,

    allocation_command_tx: Arc<
        RwLock<
            HashMap<iana_formats::addrress_family::AddressFamily, mpsc::Sender<AllocationCommond>>,
        >,
    >,
    reservation: Option<(
        attributes::rfc8656::ReservationTokenAttribute,
        UdpSocket,
        Instant,
    )>,
}

impl Session {
    pub fn new(
        protocol: Protocol,
        local_addr: SocketAddr,
        remote_addr: SocketAddr,
        local_ipv4_addr: Ipv4Addr,
        local_ipv6_addr: Option<Ipv6Addr>,
        message_tx: mpsc::Sender<(Message, Option<oneshot::Sender<()>>)>,
        message_rx: mpsc::Receiver<Message>,
        use_icmp: bool,
    ) -> Self {
        Self {
            fivetuple: FiveTuple::new(protocol, local_addr.into(), remote_addr.into()),
            allocation_set: false,
            allocation_command_tx: Default::default(),
            local_ipv4_address: local_ipv4_addr,
            local_ipv6_address: local_ipv6_addr,
            message_rx,
            message_tx,
            reservation: None,
            use_icmp,
        }
    }
    pub async fn run(mut self) -> TurnSessionResult<()> {
        let remote_addr = self.fivetuple.remote_addr().into();
        while let Some(incoming) = self.message_rx.recv().await {
            tracing::debug!("got incoming: {}", incoming);
            let now = Instant::now();
            let reservation_expire = if let Some((_, _, deadline)) = self.reservation
                && now > deadline
            {
                true
            } else {
                false
            };
            if reservation_expire {
                tracing::info!("reservation expired, {}", self.reservation.unwrap().0);
                self.reservation = None;
            }
            match incoming {
                Message::ChannelData(data) => {
                    if self.allocation_command_tx.read().await.is_empty() {
                        tracing::warn!(
                            "got channel data while not alloation is set, data: {}, remote: {}",
                            data,
                            remote_addr
                        );
                        continue;
                    }

                    let guard = self.allocation_command_tx.read().await;
                    for (family, tx) in &*guard {
                        let _ = tx
                            .send(AllocationCommond::ChannelData(data.clone()))
                            .await
                            .map_err(|err| {
                                return std::io::Error::new(
                                    std::io::ErrorKind::BrokenPipe,
                                    format!("send command to allocation failed: {}, relay ip family: {}", err, family),
                                );
                            })?;
                    }
                }
                Message::Stun(stun) => {
                    tracing::info!("process turn message: {}", stun);
                    let response =
                        self.process_message(remote_addr, &stun)
                            .await
                            .inspect_err(|err| {
                                tracing::error!(
                                    "process turn message: {} failed, err: {}",
                                    stun,
                                    err
                                );
                            })?;
                    if let Some(response) = response {
                        tracing::info!("response for {}: {}", stun, response,);
                        if let Err(err) = self.message_tx.send((response.into(), None)).await {
                            tracing::error!("connection reset by peer, err: {}", err);
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::ConnectionReset,
                                format!("connection reset by peer"),
                            )
                            .into());
                        }
                    }
                }
            }
        }
        tracing::info!("session closed");
        Ok(())
    }

    async fn allocation_event_loop(
        mut allocation_event_rx: mpsc::Receiver<AllocationEvent>,
        message_tx: mpsc::Sender<(Message, Option<oneshot::Sender<()>>)>,
    ) -> TurnSessionResult<()> {
        loop {
            let event = allocation_event_rx.recv().await;
            if event.is_none() {
                tracing::warn!(
                    "allocation event sender has been closed, the allocation must have been ended"
                );
                return Ok(());
            }
            let event = event.unwrap();
            let response = match event {
                AllocationEvent::RelayedPacket {
                    peer_addr,
                    relay_addr,
                    data,
                    channel_number,
                } => {
                    tracing::debug!(
                        "got relayed packet from peer: {}, relay addr: {}, channel_number: {:?}, data len: {}",
                        peer_addr,
                        relay_addr,
                        channel_number,
                        data.len()
                    );
                    if let Some(channel_number) = channel_number {
                        turn_formats::message::Message::ChannelData(ChannelData::new(
                            channel_number,
                            data.into(),
                        ))
                    } else {
                        let message = stun_formats::message::Message::builder()
                            .indication()
                            .method(Box::new(methods::rfc8656::DATA::new()))
                            .transaction_id(TransactionId::new_random())
                            .attribute(attributes::rfc8656::DataAttribute::new(data.into()))?
                            .attribute(attributes::rfc8656::XorPeerAddressAttribute::new(
                                peer_addr.into(),
                            ))?
                            .build()?;
                        turn_formats::message::Message::Stun(message)
                    }
                }
                AllocationEvent::ICMP {
                    source_addr: _,
                    destination_addr,
                    icmp_type,
                    icmp_code,
                    mtu,
                } => {
                    let message = stun_formats::message::Message::builder()
                        .indication()
                        .method(Box::new(methods::rfc8656::DATA::new()))
                        .transaction_id(TransactionId::new_random())
                        .attribute(attributes::rfc8656::XorPeerAddressAttribute::new(
                            destination_addr,
                        ))?
                        .attribute(attributes::rfc8656::IcmpAttribute::new(
                            icmp_type,
                            icmp_code,
                            mtu.unwrap_or(0),
                        ))?
                        .build()?;
                    turn_formats::message::Message::Stun(message)
                }
            };
            tracing::debug!("send relayed message to client: {}", response);
            let res = message_tx.send((response, None)).await.map_err(|err| {
                       return std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        format!("send message to turn connection failed: {}, the connection must have been ended", err)
                        )
                    });
            if let Err(err) = res {
                tracing::error!("{}", err);
                return Err(err.into());
            }
        }
    }

    async fn process_message(
        &mut self,
        remote_addr: SocketAddr,
        message: &stun_formats::message::Message,
    ) -> TurnSessionResult<Option<stun_formats::message::Message>> {
        let res = match message.message_method().value() {
            stun_formats::methods::rfc8489::BINDING::STATIC_VALUE => {
                let response = stun_formats::message::Message::builder()
                    .success()
                    .binding()
                    .transaction_id(message.transaction_id().clone())
                    .attribute(
                        stun_formats::attributes::rfc8489::XorMappedAddressAttribute::new(
                            remote_addr,
                        ),
                    )
                    .unwrap()
                    .attribute(
                        stun_formats::attributes::rfc8489::MappedAddressAttribute::new(remote_addr),
                    )
                    .unwrap()
                    .finger_print()
                    .unwrap()
                    .build()
                    .unwrap();
                return Ok(Some(response));
            }
            turn_formats::methods::rfc8656::ALLOCATE::STATIC_VALUE => {
                self.process_allocate(remote_addr, message).await
            }
            turn_formats::methods::rfc8656::CREATE_PERMISSION::STATIC_VALUE => {
                self.process_create_permission(remote_addr, message).await
            }
            turn_formats::methods::rfc8656::CHANNEL_BIND::STATIC_VALUE => {
                self.process_channel_binding(remote_addr, message).await
            }
            turn_formats::methods::rfc8656::REFRESH::STATIC_VALUE => {
                self.process_refresh(remote_addr, message).await
            }
            turn_formats::methods::rfc8656::SEND::STATIC_VALUE => {
                self.process_send(remote_addr, message).await
            }
            _ => Ok(None),
        };
        match res {
            Err(err) => {
                tracing::warn!(
                    "trying to convert err into stun error response, err: {}",
                    err
                );
                let mut error_builder = stun_formats::message::Message::builder()
                    .transaction_id(message.transaction_id().clone())
                    .method(message.message_method().clone_box());
                err.try_prepare_error_response(&mut error_builder)?;
                let response = error_builder.build()?;
                Ok(Some(response))
            }
            _ => res,
        }
    }

    async fn process_send(
        &mut self,
        _remote_addr: SocketAddr,
        message: &stun_formats::message::Message,
    ) -> TurnSessionResult<Option<stun_formats::message::Message>> {
        if !matches!(message.message_class(), MessageClass::Indication) {
            return Err(crate::errors::TurnSessionError::ExpectIndication(
                message.clone(),
            ));
        }
        debug_assert_eq!(
            message.message_method().value(),
            turn_formats::methods::rfc8656::SEND::STATIC_VALUE
        );
        if !self.allocation_set {
            tracing::warn!(
                "got send message while no allocation is set, message: {}",
                message
            );
            return Ok(None);
        }
        debug_assert!(!self.allocation_command_tx.read().await.is_empty());
        let peer_addr = message
            .get_attribute_ext::<turn_formats::attributes::rfc8656::XorPeerAddressAttribute>();
        let data = message.get_attribute_ext::<turn_formats::attributes::rfc8656::DataAttribute>();
        if peer_addr.is_none() || data.is_none() {
            tracing::warn!("got ill formed send message: {}", message);
            return Ok(None);
        }
        let dont_fragment =
            message.get_attribute_ext::<turn_formats::attributes::rfc8656::DontFragmentAttribute>();
        let peer_addr = peer_addr.unwrap();
        tracing::debug!("send indication with peer_addr: {}", peer_addr);
        let peer = peer_addr.address();
        let family = iana_formats::addrress_family::for_address(&peer);
        if let Some(tx) = self.allocation_command_tx.read().await.get(&family) {
            tx.send(AllocationCommond::Send {
                peer_addr: peer_addr.address(),
                data: data.unwrap().data(),
                require_no_fragment: dont_fragment.is_some(),
            })
            .await
            .map_err(|err| {
                return std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    format!("send command to allocation failed: {}", err),
                );
            })?;
        } else {
            tracing::warn!(
                "got send indication but requested peer addr family is not supported, messag: {}, peer: {}",
                message,
                peer
            );
        }
        Ok(None)
    }

    async fn process_refresh(
        &mut self,
        remote_addr: SocketAddr,
        message: &stun_formats::message::Message,
    ) -> TurnSessionResult<Option<stun_formats::message::Message>> {
        if !matches!(message.message_class(), MessageClass::Request) {
            return Err(crate::errors::TurnSessionError::ExpectRequest(
                message.clone(),
            ));
        }
        debug_assert_eq!(
            message.message_method().value(),
            turn_formats::methods::rfc8656::REFRESH::STATIC_VALUE
        );
        if !self.allocation_set {
            return Err(TurnSessionError::AllocationMismatch {
                remote: remote_addr,
                local: self.fivetuple.local_addr().into(),
            });
        }
        debug_assert!(!self.allocation_command_tx.read().await.is_empty());
        let lifetime = message
            .get_attribute_ext::<attributes::rfc8656::LifeTimeAttribute>()
            .map(|item| item.lifetime());
        let address_family = message
            .get_attribute_ext::<attributes::rfc8656::RequestedAddressFamilyAttribute>()
            .map(|item| item.family());
        let mut duration = None;
        for (family, tx) in &*self.allocation_command_tx.read().await {
            if address_family.is_none() || &address_family.unwrap() == family {
                let (result_tx, result_rx) = oneshot::channel();
                let command = AllocationCommond::Refresh {
                    lifetime,
                    address_family,
                    result_tx,
                };
                tx.send(command).await.map_err(|err| {
                    return std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        format!("send command to allocation failed: {}", err),
                    );
                })?;
                duration = Some(result_rx.await.map_err(|err| {
                    return std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        format!("error recv command result from allocation: {}", err),
                    );
                })??);
            }
        }
        if duration.is_none() && address_family.is_some() {
            return Err(TurnSessionError::UnsupportedAddressFamily(
                address_family.unwrap(),
            ));
        }
        return Ok(Some(
            stun_formats::message::Message::builder()
                .success()
                .method(message.message_method().clone_box())
                .transaction_id(*message.transaction_id())
                .attribute(attributes::rfc8656::LifeTimeAttribute::new(
                    duration.unwrap_or(Duration::ZERO),
                ))?
                .build()?,
        ));
    }

    async fn process_channel_binding(
        &mut self,
        remote_addr: SocketAddr,
        message: &stun_formats::message::Message,
    ) -> TurnSessionResult<Option<stun_formats::message::Message>> {
        if !matches!(message.message_class(), MessageClass::Request) {
            return Err(crate::errors::TurnSessionError::ExpectRequest(
                message.clone(),
            ));
        }
        debug_assert_eq!(
            message.message_method().value(),
            turn_formats::methods::rfc8656::CHANNEL_BIND::STATIC_VALUE
        );
        if !self.allocation_set {
            return Err(TurnSessionError::AllocationMismatch {
                remote: remote_addr,
                local: self.fivetuple.local_addr().into(),
            });
        }
        debug_assert!(!self.allocation_command_tx.read().await.is_empty());
        let channel_number = message
            .get_attribute_ext::<attributes::rfc8656::ChannelNumberAttribute>()
            .map(|item| item.channel_number);
        debug_assert!(channel_number.is_some());
        let peer_addr = message
            .get_attribute_ext::<attributes::rfc8656::XorPeerAddressAttribute>()
            .map(|item| item.address());
        debug_assert!(peer_addr.is_some());
        let peer_addr = peer_addr.unwrap();

        let peer_family = iana_formats::addrress_family::for_address(&peer_addr);
        if let Some(tx) = self.allocation_command_tx.read().await.get(&peer_family) {
            let (result_tx, result_rx) = oneshot::channel();

            let command = AllocationCommond::CreateChannelBinding {
                channel_number: channel_number.unwrap(),
                peer_addr: peer_addr.into(),
                result_tx,
            };

            tx.send(command).await.map_err(|err| {
                return std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    format!("send command to allocation failed: {}", err),
                );
            })?;
            let _ = result_rx.await.map_err(|err| {
                return std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    format!("error recv command result from allocation: {}", err),
                );
            })??;
            return Ok(Some(
                stun_formats::message::Message::builder()
                    .success()
                    .method(message.message_method().clone_box())
                    .transaction_id(*message.transaction_id())
                    .build()?,
            ));
        } else {
            return Err(TurnSessionError::PeerAddressFamilyNotMatch(peer_family));
        }
    }

    async fn process_create_permission(
        &mut self,
        remote_addr: SocketAddr,
        message: &stun_formats::message::Message,
    ) -> TurnSessionResult<Option<stun_formats::message::Message>> {
        if !matches!(message.message_class(), MessageClass::Request) {
            return Err(crate::errors::TurnSessionError::ExpectRequest(
                message.clone(),
            ));
        }
        debug_assert_eq!(
            message.message_method().value(),
            turn_formats::methods::rfc8656::CREATE_PERMISSION::STATIC_VALUE
        );
        if !self.allocation_set {
            return Err(TurnSessionError::AllocationMismatch {
                remote: remote_addr,
                local: self.fivetuple.local_addr().into(),
            });
        }
        debug_assert!(!self.allocation_command_tx.read().await.is_empty());

        let address: Vec<_> = message
            .attributes_ext::<attributes::rfc8656::XorPeerAddressAttribute>()
            .map(|item| item.ip())
            .collect();
        debug_assert!(!address.is_empty());
        let probe_address_family = iana_formats::addrress_family::for_ip_address(&address[0]);
        if let Some(tx) = self
            .allocation_command_tx
            .read()
            .await
            .get(&probe_address_family)
        {
            let (result_tx, result_rx) = oneshot::channel();
            let command = AllocationCommond::CreatePermission {
                peer_ips: address,
                result_tx,
            };
            tx.send(command).await.map_err(|err| {
                return std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    format!("send command to allocation failed: {}", err),
                );
            })?;
            let _ = result_rx.await.map_err(|err| {
                return std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    format!("error recv command result from allocation: {}", err),
                );
            })??;
            return Ok(Some(
                stun_formats::message::Message::builder()
                    .success()
                    .method(message.message_method().clone_box())
                    .transaction_id(*message.transaction_id())
                    .build()?,
            ));
        } else {
            return Err(TurnSessionError::PeerAddressFamilyNotMatch(
                probe_address_family,
            ));
        }
    }

    async fn process_allocate(
        &mut self,
        remote_addr: SocketAddr,
        message: &stun_formats::message::Message,
    ) -> TurnSessionResult<Option<stun_formats::message::Message>> {
        if !matches!(message.message_class(), MessageClass::Request) {
            return Err(crate::errors::TurnSessionError::ExpectRequest(
                message.clone(),
            ));
        }
        debug_assert_eq!(
            message.message_method().value(),
            turn_formats::methods::rfc8656::ALLOCATE::STATIC_VALUE
        );
        let reservation_token =
            message.get_attribute_ext::<attributes::rfc8656::ReservationTokenAttribute>();
        if reservation_token.is_some() {
            let reservation_match = self.reservation.is_some()
                && self.reservation.as_ref().unwrap().0.token()
                    == reservation_token.unwrap().token();
            if !reservation_match {
                return Err(TurnSessionError::InvalidReservationToken(
                    reservation_token.unwrap(),
                ));
            }
            let (_, socket, _) = self.reservation.take().unwrap();
            let allocation = Allocation::builder()
                .local_addr(self.fivetuple.local_addr().into())
                .local_ipv4_addr(self.local_ipv4_address)
                .local_ipv6_addr(self.local_ipv6_address)
                .remote_addr(remote_addr)
                .with_reservation(Some(socket))
                .build()
                .await?;
            let relay_addr = allocation.relayed_transport_address();
            self.setup_allocation(allocation).await?;
            let response = stun_formats::message::Message::builder()
                .transaction_id(message.transaction_id().clone())
                .success()
                .method_ext(turn_formats::methods::rfc8656::ALLOCATE)
                .attribute(turn_formats::attributes::rfc8656::LifeTimeAttribute::new(
                    ALLOCATION_LIFETIME,
                ))?
                .attribute(
                    turn_formats::attributes::rfc8656::XorRelayedAddressAttribute::new(relay_addr),
                )?
                .attribute(
                    stun_formats::attributes::rfc8489::XorMappedAddressAttribute::new(remote_addr),
                )?
                .build()?;
            return Ok(Some(response));
        }
        let protocol = message
            .get_attribute_ext::<attributes::rfc8656::RequestedTransportAttribute>()
            .expect(&format!(
                "{} attribute is required",
                attributes::rfc8656::RequestedTransportAttribute::STATIC_NAME,
            ));
        let protocol_value = protocol.value();
        let protocol = protocol
            .protocol()
            .ok_or(TurnSessionError::UnknownProtocol(protocol_value))?;
        let additional = message.count(
            turn_formats::attributes::rfc8656::AdditionalAddressFamilyAttribute::STATIC_ATTR_TYPE,
        ) > 0;
        let required_address_family = message
            .get_attribute_ext::<turn_formats::attributes::rfc8656::RequestedAddressFamilyAttribute>()
            .map(|item| item.family())
            .unwrap_or(iana_formats::addrress_family::IPv4::ADDRESS_FAMILY);
        let even_port =
            message.get_attribute_ext::<turn_formats::attributes::rfc8656::EvenPortAttribute>();
        let require_even_port = even_port.is_some();
        let require_reservation = even_port.map(|item| item.r).unwrap_or(false);

        let all_address_families = [
            iana_formats::addrress_family::IPv4::ADDRESS_FAMILY,
            iana_formats::addrress_family::IPv6::ADDRESS_FAMILY,
        ];
        let mut relay_addresses = vec![];
        for family in all_address_families {
            if additional || required_address_family == family {
                let allocation = Allocation::builder()
                    .local_addr(self.fivetuple.local_addr().into())
                    .local_ipv4_addr(self.local_ipv4_address)
                    .local_ipv6_addr(self.local_ipv6_address)
                    .remote_addr(remote_addr)
                    .relay_address_family(Some(family))?
                    .protocol(protocol.protocol())?
                    .require_even_port(require_even_port)
                    .build()
                    .await?;
                let relay_address = allocation.relayed_transport_address();
                if require_reservation {
                    debug_assert!(!additional);
                    let max_port = relay_address.port();
                    let reservation = self.try_make_reservation(max_port, family)?;

                    self.reservation = Some((
                        attributes::rfc8656::ReservationTokenAttribute::new_random(),
                        reservation,
                        Instant::now().checked_add(RESERVATION_LIFETIME).unwrap(),
                    ));
                }
                relay_addresses.push(relay_address);
                self.setup_allocation(allocation).await?;
            }
        }

        debug_assert!(!relay_addresses.is_empty());

        let mut response = stun_formats::message::Message::builder()
            .transaction_id(message.transaction_id().clone())
            .success()
            .method_ext(turn_formats::methods::rfc8656::ALLOCATE)
            .attribute(turn_formats::attributes::rfc8656::LifeTimeAttribute::new(
                ALLOCATION_LIFETIME,
            ))?;
        for relay_addr in relay_addresses {
            response = response.attribute(
                turn_formats::attributes::rfc8656::XorRelayedAddressAttribute::new(relay_addr),
            )?;
        }
        if let Some((token, ..)) = self.reservation.as_ref() {
            response = response.attribute(token.clone())?;
        }

        let response = response
            .attribute(
                stun_formats::attributes::rfc8489::XorMappedAddressAttribute::new(remote_addr),
            )?
            .build()?;

        Ok(Some(response))
    }

    async fn setup_allocation(&mut self, allocation: Allocation) -> TurnSessionResult<()> {
        let relay_address = allocation.relayed_transport_address();
        let relay_family = iana_formats::addrress_family::for_address(&relay_address);
        let (command_tx, command_rx) = mpsc::channel(100);
        let (event_tx, event_rx) = mpsc::channel(100);
        self.allocation_command_tx
            .write()
            .await
            .insert(relay_family, command_tx);
        let command_tx_for_allocation = self.allocation_command_tx.clone();
        tokio::spawn(async move {
            let _ = allocation
                .run(command_rx, event_tx)
                .await
                .inspect_err(|err| tracing::warn!("allocation exit with err: {}", err));
            command_tx_for_allocation
                .write()
                .await
                .remove(&relay_family);
            tracing::info!("allocation ended");
        });
        let message_tx_for_allocation_event = self.message_tx.clone();
        tokio::spawn(async move {
            let res = Self::allocation_event_loop(event_rx, message_tx_for_allocation_event).await;
            tracing::info!("allocation event handle loop ended: {:?}", res);
        });
        self.allocation_set = true;
        Ok(())
    }

    fn try_make_reservation(
        &self,
        maxport: u16,
        family: iana_formats::addrress_family::AddressFamily,
    ) -> TurnSessionResult<UdpSocket> {
        debug_assert!(maxport.is_multiple_of(2));
        let next_port = maxport + 2;
        let addr = if family == iana_formats::addrress_family::IPv4::ADDRESS_FAMILY {
            SocketAddr::new(IpAddr::V4(self.local_ipv4_address), next_port)
        } else {
            if let Some(local_ipv6_addr) = self.local_ipv6_address {
                SocketAddr::new(IpAddr::V6(local_ipv6_addr), next_port)
            } else {
                return Err(TurnSessionError::UnsupportedAddressFamily(
                    iana_formats::addrress_family::IPv6::ADDRESS_FAMILY,
                ));
            }
        };
        let socket = socket2::Socket::new(
            Domain::for_address(addr),
            Type::DGRAM,
            Some(socket2::Protocol::UDP),
        )?;
        socket.set_reuse_address(true)?;
        socket.set_reuse_port(true)?;
        socket.set_nonblocking(true)?;
        socket.bind(&addr.into())?;
        let port = socket.local_addr()?.as_socket().unwrap().port();
        if port != next_port {
            return Err(TurnSessionError::MakeReservationFailed(next_port));
        }
        let std_socket: std::net::UdpSocket = socket.into();
        let udp_socket = tokio::net::UdpSocket::from_std(std_socket)?;
        Ok(udp_socket)
    }
}
