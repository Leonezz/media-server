use std::{fmt, net::SocketAddr};

use iana_formats::protocol_numbers::{Protocol, ProtocolNumberStatic};

#[derive(PartialEq, Eq, Clone, Hash)]
pub struct FiveTuple {
    protocol: iana_formats::protocol_numbers::Protocol,
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
}

impl FiveTuple {
    pub fn new(
        protocol: iana_formats::protocol_numbers::Protocol,
        local_addr: SocketAddr,
        remote_addr: SocketAddr,
    ) -> Self {
        Self {
            protocol,
            local_addr,
            remote_addr,
        }
    }

    pub fn new_udp(local_addr: SocketAddr, remote_addr: SocketAddr) -> Self {
        Self::new(
            iana_formats::protocol_numbers::UDP::PROTOCOL,
            local_addr,
            remote_addr,
        )
    }

    pub fn new_tcp(local_addr: SocketAddr, remote_addr: SocketAddr) -> Self {
        Self::new(
            iana_formats::protocol_numbers::TCP::PROTOCOL,
            local_addr,
            remote_addr,
        )
    }

    pub fn protocol(&self) -> Protocol {
        self.protocol
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }
}

impl fmt::Debug for FiveTuple {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "local({:?}) <--{}--> remote({:?})",
            self.local_addr(),
            iana_formats::protocol_numbers::from_protocol(self.protocol())
                .map(|item| item.keyword())
                .unwrap_or("unknown protocol"),
            self.remote_addr()
        )
    }
}

#[cfg(test)]
mod test {
    use std::net::IpAddr;

    use crate::five_tuple::{FiveTuple, SocketAddr};

    #[test]
    fn test_fmt_debug_address() {
        let ip: IpAddr = "0.0.0.1".parse().unwrap();
        let address = SocketAddr::new(ip, 20);
        println!("{:?}", address);
        assert_eq!(format!("{:?}", address), "[0.0.0.1]:20");
    }

    #[test]
    fn test_fmt_debug_five_tuple() {
        let local: IpAddr = "1.2.0.1".parse().unwrap();
        let remote: IpAddr = "222.111.222.111".parse().unwrap();
        let five_tuple =
            FiveTuple::new_udp(SocketAddr::new(local, 30), SocketAddr::new(remote, 100));
        println!("{:?}", five_tuple);
        assert_eq!(
            format!("{:?}", five_tuple),
            "local([1.2.0.1]:30) <--UDP--> remote([222.111.222.111]:100)"
        );
    }
}
