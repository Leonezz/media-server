use clap::{Parser, builder::NonEmptyStringValueParser};
use iana_formats::protocol_numbers::ProtocolNumberStatic;
use std::{
    default,
    net::{IpAddr, Ipv4Addr},
};
use utils::config::AppCliWithLogger;

const DEFAULT_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
const DEFAULT_PORT: u16 = 0;
const DEFAULT_SERVER: &'static str = "stun.l.google.com:19302";

#[derive(Debug, Clone, Parser, serde::Serialize, serde::Deserialize)]
pub struct StunClientCli {
    #[arg(
        long,
        env,
        value_name = "SERVER",
        help = "stun server address trying to reach",
        ignore_case = true,
        value_parser = NonEmptyStringValueParser::new(),
        default_value = DEFAULT_SERVER
    )]
    pub server: String,
    #[arg(
        long,
        env,
        value_name = "PROTOCOL",
        ignore_case = true,
        help = "transport protocol to use for stun transaction, tcp or udp",
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

impl std::fmt::Display for StunClientCli {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl default::Default for StunClientCli {
    fn default() -> Self {
        Self {
            server: DEFAULT_SERVER.to_owned(),
            protocol: iana_formats::protocol_numbers::UDP::PROTOCOL,
            local_addr: DEFAULT_ADDRESS,
            local_port: DEFAULT_PORT,
        }
    }
}

pub type AppCli = utils::config::AppCli<AppCliWithLogger<StunClientCli>>;
