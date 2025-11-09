use clap::Parser;
use iana_formats::{protocol_numbers::ProtocolNumberStatic, service_port::ServicePortStatic};
use std::{
    default,
    net::{IpAddr, Ipv4Addr},
};

const DEFAULT_PROTOCOL: iana_formats::protocol_numbers::Protocol =
    iana_formats::protocol_numbers::UDP::PROTOCOL;
const DEFAULT_PORT: u16 = iana_formats::service_port::STUN_UDP::PORT.min_port();
const DEFAULT_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);

#[derive(Debug, Clone, Parser, serde::Deserialize, serde::Serialize)]
pub struct StunServerCli {
    #[arg(
        long,
        env,
        value_name = "PROTOCOL",
        ignore_case = true,
        help = "transport protocol to use for listening, tcp or udp",
        value_parser = utils::config::protocol_parser(&["tcp", "udp"]),
        default_value = "udp"
    )]
    pub protocol: iana_formats::protocol_numbers::Protocol,
    #[arg(
        long,
        env,
        value_name = "LOCAL_ADDR",
        ignore_case = true,
        help = "ip address to use for listening",
        value_parser = clap::value_parser!(IpAddr),
        default_value = DEFAULT_ADDRESS.to_string()
    )]
    pub local_addr: IpAddr,
    #[arg(
        long,
        env,
        value_name = "LOCAL_PORT",
        ignore_case = true,
        value_parser = clap::value_parser!(u16).range(u16::MIN as i64..u16::MAX as i64),
        default_value = DEFAULT_PORT.to_string()
    )]
    pub local_port: u16,
}

impl std::fmt::Display for StunServerCli {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl default::Default for StunServerCli {
    fn default() -> Self {
        Self {
            protocol: DEFAULT_PROTOCOL,
            local_addr: DEFAULT_ADDRESS,
            local_port: DEFAULT_PORT,
        }
    }
}

impl StunServerCli {
    pub fn resolve(mut self) -> Self {
        if self.local_port == DEFAULT_PORT {
            if matches!(self.protocol, iana_formats::protocol_numbers::TCP::PROTOCOL) {
                self.local_port = iana_formats::service_port::STUN_TCP::PORT.min_port();
            } else if matches!(self.protocol, iana_formats::protocol_numbers::UDP::PROTOCOL) {
                self.local_port = iana_formats::service_port::STUN_UDP::PORT.min_port();
            }
        }
        self
    }
}

pub type AppCli = utils::config::AppCli<utils::config::AppCliWithLogger<StunServerCli>>;
