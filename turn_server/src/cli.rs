use clap::Parser;
use iana_formats::{
    addrress_family::AddressFamilyStatic, protocol_numbers::ProtocolNumberStatic,
    service_port::ServicePortStatic,
};
use serde::{Deserialize, Serialize};
use std::{
    default,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

const DEFAULT_LOCAL_ADDR: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
const DEFAULT_LOCAL_PORT: u16 = iana_formats::service_port::TURN_UDP::PORT.min_port();

#[derive(Debug, Parser, Deserialize, Serialize)]
pub struct TurnServerCli {
    #[arg(
        long,
        env,
        value_name = "PROTOCOL",
        help = "transport protocol to use for listening, tcp or udp",
        ignore_case = true,
        value_parser = utils::config::protocol_parser(&["tcp", "udp"]),
        default_value = "udp"
    )]
    pub protocol: iana_formats::protocol_numbers::Protocol,
    #[arg(
        long,
        env,
        value_name = "IP_FAMILY",
        ignore_case = true,
        help = "ip family to ues for listening, ipv4 or ipv6",
        value_parser = utils::config::ip_family_parser(&["ipv4", "ipv6"]),
        default_value = "ipv4"
    )]
    pub family: iana_formats::addrress_family::AddressFamily,
    #[arg(
        long,
        env,
        value_name = "LOCAL_ADDR",
        help = "local ip address to use for listening",
        value_parser = clap::value_parser!(IpAddr),
        ignore_case = true,
        default_value = DEFAULT_LOCAL_ADDR.to_string()
    )]
    pub localaddr: IpAddr,
    #[arg(
        long,
        env,
        value_name = "LOCAL_PORT",
        help = "local port to use for listening",
        value_parser = clap::value_parser!(u16).range(u16::MIN as i64..u16::MAX as i64),
        ignore_case = true,
        default_value = DEFAULT_LOCAL_PORT.to_string(),
    )]
    pub localport: u16,
    #[arg(
        long,
        env,
        value_name = "USE_ICMP",
        help = "enable icmp endpoint for relay, requires admin permission",
        ignore_case = true,
        default_value = "false"
    )]
    pub use_icmp: bool,
    #[arg(
        long,
        env,
        value_name = "LOCAL_IPV4_ADDR",
        help = "local ipv4 address used for relay",
        ignore_case = true,
        default_value = Ipv4Addr::UNSPECIFIED.to_string()
    )]
    pub local_ipv4_addr: Ipv4Addr,
    #[arg(
        long,
        env,
        value_name = "USE_IPV6",
        help = "enable ipv6 address for relay",
        ignore_case = true,
        default_value = "false"
    )]
    pub use_ipv6: bool,
    #[arg(
        long,
        env,
        value_name = "LOCAL_IPV6_ADDR",
        help = "local ipv6 address used for relay",
        ignore_case = true,
        default_value = Ipv6Addr::UNSPECIFIED.to_string()
    )]
    pub local_ipv6_addr: Ipv6Addr,
}

impl default::Default for TurnServerCli {
    fn default() -> Self {
        Self {
            protocol: iana_formats::protocol_numbers::UDP::PROTOCOL,
            family: iana_formats::addrress_family::IPv4::ADDRESS_FAMILY,
            localaddr: DEFAULT_LOCAL_ADDR,
            localport: DEFAULT_LOCAL_PORT,
            use_icmp: false,
            local_ipv4_addr: Ipv4Addr::UNSPECIFIED,
            use_ipv6: false,
            local_ipv6_addr: Ipv6Addr::UNSPECIFIED,
        }
    }
}

impl TurnServerCli {
    // for default_value_if
    pub fn resolve(mut self) -> Self {
        if self.localport == DEFAULT_LOCAL_PORT {
            if matches!(self.protocol, iana_formats::protocol_numbers::TCP::PROTOCOL) {
                self.localport = iana_formats::service_port::TURN_TCP::PORT.min_port();
            } else if matches!(self.protocol, iana_formats::protocol_numbers::UDP::PROTOCOL) {
                self.localport = iana_formats::service_port::TURN_UDP::PORT.min_port();
            }
        }

        if self.localaddr == DEFAULT_LOCAL_ADDR {
            if matches!(
                self.family,
                iana_formats::addrress_family::IPv6::ADDRESS_FAMILY
            ) {
                self.localaddr = IpAddr::V6(Ipv6Addr::UNSPECIFIED);
            } else if matches!(
                self.family,
                iana_formats::addrress_family::IPv4::ADDRESS_FAMILY
            ) {
                self.localaddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
            }
        }

        self
    }
}

impl std::fmt::Display for TurnServerCli {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub type AppCli = utils::config::AppCli<utils::config::AppCliWithLogger<TurnServerCli>>;
