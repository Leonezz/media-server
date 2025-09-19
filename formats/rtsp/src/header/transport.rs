use std::{fmt, str::FromStr};

use num::Integer;

use crate::errors::RtspMessageError;

#[derive(Clone, Copy)]
pub enum TransportProtocol {
    RtpAvpUdp,
    RtpAvpTcp,
    RtpAvpfUdp,
    RtpAvpfTcp,
    RtpSavpUdp,
    RtpSavpTcp,
    RtpSavpfUdp,
    RtpSavpfTcp,
}

impl TransportProtocol {
    pub fn is_udp(&self) -> bool {
        matches!(
            self,
            Self::RtpAvpUdp | Self::RtpAvpfUdp | Self::RtpSavpUdp | Self::RtpSavpfUdp
        )
    }

    pub fn is_tcp(&self) -> bool {
        matches!(
            self,
            Self::RtpAvpTcp | Self::RtpAvpfTcp | Self::RtpSavpTcp | Self::RtpSavpfTcp
        )
    }
}

impl fmt::Display for TransportProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RtpAvpUdp => write!(f, "RTP/AVP/UDP"),
            Self::RtpAvpTcp => write!(f, "RTP/AVP/TCP"),
            Self::RtpAvpfUdp => write!(f, "RTP/AVPF/UDP"),
            Self::RtpAvpfTcp => write!(f, "RTP/AVPF/TCP"),
            Self::RtpSavpUdp => write!(f, "RTP/SAVP/UDP"),
            Self::RtpSavpTcp => write!(f, "RTP/SAVP/TCP"),
            Self::RtpSavpfUdp => write!(f, "RTP/SAVPF/UDP"),
            Self::RtpSavpfTcp => write!(f, "RTP/SAVPF/TCP"),
        }
    }
}

impl fmt::Debug for TransportProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Clone)]
pub enum TransportMode {
    Play,
    Record,
    Other(String),
}

impl fmt::Display for TransportMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Play => write!(f, "PLAY"),
            Self::Record => write!(f, "RECORD"),
            Self::Other(str) => write!(f, "{}", str),
        }
    }
}

impl fmt::Debug for TransportMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Clone, Copy)]
pub enum TransportCast {
    Unicast,
    Multicast,
}

impl fmt::Display for TransportCast {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unicast => write!(f, "unicast"),
            Self::Multicast => write!(f, "multicast"),
        }
    }
}

impl fmt::Debug for TransportCast {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Clone)]
pub enum Addr {
    Port(u16),
    Host(String),
    HostPort((String, u16)),
}

impl fmt::Display for Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Port(port) => write!(f, "{}", port),
            Self::Host(host) => write!(f, "{}", host),
            Self::HostPort((host, port)) => write!(f, "{}:{}", host, port),
        }
    }
}

impl fmt::Debug for Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

fn parse_number_range<T: FromStr + Integer + Copy>(s: &str) -> Result<(T, T), T::Err> {
    if !s.contains('-') {
        let port: T = s.parse::<T>()?;
        return Ok((port, port));
    }

    let (first, second) = s.split_once('-').unwrap();
    Ok((first.parse()?, second.parse()?))
}

impl FromStr for Addr {
    type Err = RtspMessageError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.contains(':') {
            return Ok(Self::Host(s.to_owned()));
        }

        let (first, second) = s.split_once(':').unwrap();
        let port = second.parse::<u16>().map_err(|err| {
            RtspMessageError::InvalidRtspMessageFormat(format!(
                "parse u16 port number failed: {}, {}",
                second, err
            ))
        })?;
        if first.is_empty() {
            return Ok(Self::Port(port));
        }

        Ok(Self::HostPort((first.to_owned(), port)))
    }
}

#[derive(Clone, Copy)]
pub enum Setup {
    Active,
    Passive,
    Actpass,
}

impl fmt::Display for Setup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Passive => write!(f, "passive"),
            Self::Actpass => write!(f, "actpass"),
        }
    }
}

impl fmt::Debug for Setup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Clone, Copy)]
pub enum Connection {
    New,
    Existing,
}

impl fmt::Display for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::New => write!(f, "new"),
            Self::Existing => write!(f, "existing"),
        }
    }
}

impl fmt::Debug for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Debug, Default, Clone)]
pub struct TransportHeader {
    pub profile: Option<TransportProtocol>,
    pub cast: Option<TransportCast>,
    pub interleaved: Option<(u8, u8)>,
    pub ttl: Option<u8>,
    pub layers: Option<u8>,
    pub ssrc_list: Vec<u32>,
    pub mode: Vec<TransportMode>,
    pub dest_addr: Vec<Addr>,
    pub src_addr: Vec<Addr>,
    pub setup: Option<Setup>,
    pub connection: Option<Connection>,
    pub rtcp_mux: bool,
    pub mikey: Option<String>,

    pub client_port: Option<(u16, u16)>,
    pub server_port: Option<(u16, u16)>,
    pub port: Option<(u16, u16)>,
    pub append: bool,
    pub destination: Option<String>,
}

impl fmt::Display for TransportHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut result = Vec::new();
        if let Some(profile) = &self.profile {
            result.push(format!("{}", profile));
        }
        if let Some(cast) = &self.cast {
            result.push(format!("{}", cast));
        }
        if let Some(interleaved) = &self.interleaved {
            result.push(format!("interleaved={}-{}", interleaved.0, interleaved.1));
        }
        if let Some(ttl) = &self.ttl {
            result.push(format!("ttl={}", ttl));
        }
        if let Some(layers) = &self.layers {
            result.push(format!("layers={}", layers));
        }
        if !self.ssrc_list.is_empty() {
            result.push(format!(
                "ssrc={}",
                self.ssrc_list
                    .iter()
                    .map(|ssrc| ssrc.to_string())
                    .collect::<Vec<String>>()
                    .join("/")
            ));
        }
        if !self.mode.is_empty() {
            result.push(format!(
                "mode={}",
                self.mode
                    .iter()
                    .map(|mode| mode.to_string())
                    .collect::<Vec<String>>()
                    .join(",")
            ));
        }
        if !self.dest_addr.is_empty() {
            result.push(format!(
                "dest_addr={}",
                self.dest_addr
                    .iter()
                    .map(|addr| addr.to_string())
                    .collect::<Vec<String>>()
                    .join("/")
            ));
        }
        if !self.src_addr.is_empty() {
            result.push(format!(
                "src_addr={}",
                self.src_addr
                    .iter()
                    .map(|addr| addr.to_string())
                    .collect::<Vec<String>>()
                    .join("/")
            ));
        }
        if let Some(setup) = &self.setup {
            result.push(format!("setup={}", setup));
        }
        if let Some(connection) = &self.connection {
            result.push(format!("connection={}", connection));
        }
        if self.rtcp_mux {
            result.push("RTCP-mux".to_string());
        }
        if let Some(mikey) = &self.mikey {
            result.push(format!("MIKEY={}", mikey));
        }
        if let Some(client_port) = &self.client_port {
            result.push(format!("client_port={}-{}", client_port.0, client_port.1));
        }
        if let Some(server_port) = &self.server_port {
            result.push(format!("server_port={}-{}", server_port.0, server_port.1));
        }
        if let Some(port) = &self.port {
            result.push(format!("port={}-{}", port.0, port.1));
        }
        if self.append {
            result.push("append".to_string());
        }
        if let Some(destination) = &self.destination {
            result.push(format!("destination={}", destination));
        }
        write!(f, "{}", result.join(";"))
    }
}

impl FromStr for TransportHeader {
    type Err = RtspMessageError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut result = Self::default();
        let params: Vec<&str> = s.split(';').collect();
        for param in params {
            let (k, v) = param.split_once('=').unwrap_or((param, ""));
            match k {
                "RTP/AVP" | "RTP/AVP/UDP" => result.profile = Some(TransportProtocol::RtpAvpUdp),
                "RTP/AVP/TCP" => result.profile = Some(TransportProtocol::RtpAvpTcp),
                "RTP/AVPF" | "RTP/AVPF/UDP" => result.profile = Some(TransportProtocol::RtpAvpfUdp),
                "RTP/AVPF/TCP" => result.profile = Some(TransportProtocol::RtpAvpfTcp),
                "RTP/SAVP" | "RTP/SAVP/UDP" => result.profile = Some(TransportProtocol::RtpSavpUdp),
                "RTP/SAVP/TCP" => result.profile = Some(TransportProtocol::RtpSavpTcp),
                "RTP/SAVPF" | "RTP/SAVPF/UDP" => {
                    result.profile = Some(TransportProtocol::RtpSavpfUdp)
                }
                "RTP/SAVPF/TCP" => result.profile = Some(TransportProtocol::RtpSavpfTcp),
                "unicast" => result.cast = Some(TransportCast::Unicast),
                "multicast" => result.cast = Some(TransportCast::Multicast),
                "layers" => {
                    result.layers = Some(v.parse::<u8>().map_err(|err| {
                        RtspMessageError::InvalidRtspMessageFormat(format!(
                            "[transport header] parsed layers as u8 value failed, {}, {}",
                            v, err
                        ))
                    })?)
                }
                "dest_addr" => {
                    for addr in v.split('/') {
                        result.dest_addr.push(addr.parse()?);
                    }
                }
                "src_addr" => {
                    for addr in v.split('/') {
                        result.src_addr.push(addr.parse()?);
                    }
                }
                "mode" => {
                    for mode in v.split(',') {
                        match mode.trim().to_uppercase().as_str() {
                            "PLAY" | "\"PLAY\"" => result.mode.push(TransportMode::Play),
                            "RECORD" | "\"RECORD\"" => result.mode.push(TransportMode::Record),
                            _ => result.mode.push(TransportMode::Other(v.to_owned())),
                        }
                    }
                }
                "interleaved" => {
                    result.interleaved = Some(parse_number_range::<u8>(v).map_err(|err| {
                        RtspMessageError::InvalidRtspMessageFormat(format!(
                            "[transport header] parse interleaved failed: {}, {}",
                            v, err
                        ))
                    })?)
                }
                "MIKEY" => result.mikey = Some(v.to_owned()),
                "ttl" => {
                    result.ttl = Some(v.parse().map_err(|err| {
                        RtspMessageError::InvalidRtspMessageFormat(format!(
                            "[transport header] parse ttl failed: {}, {}",
                            v, err,
                        ))
                    })?);
                }
                "ssrc" => {
                    for ssrc in v.split('/') {
                        result.ssrc_list.push(ssrc.parse().map_err(|err| {
                            RtspMessageError::InvalidRtspMessageFormat(format!(
                                "[transport header] parse ssrc failed: {}, {}",
                                v, err
                            ))
                        })?);
                    }
                }
                "RTCP-mux" => result.rtcp_mux = true,
                "setup" => match v {
                    "active" => result.setup = Some(Setup::Active),
                    "passive" => result.setup = Some(Setup::Passive),
                    "actpass" => result.setup = Some(Setup::Actpass),
                    _ => {
                        return Err(RtspMessageError::InvalidRtspMessageFormat(format!(
                            "[transport header] invalid setup: {}",
                            v
                        )));
                    }
                },
                "connection" => match v {
                    "new" => result.connection = Some(Connection::New),
                    "existing" => result.connection = Some(Connection::Existing),
                    _ => {
                        return Err(RtspMessageError::InvalidRtspMessageFormat(format!(
                            "[transport header] invalid connection: {}",
                            v
                        )));
                    }
                },

                "destination" => {
                    if !v.is_empty() {
                        result.destination = Some(v.to_owned())
                    }
                }
                "append" => {
                    result.append = true;
                }
                "port" => {
                    result.port = Some(parse_number_range::<u16>(v).map_err(|err| {
                        RtspMessageError::InvalidRtspMessageFormat(format!(
                            "[transport header] parse port range for port failed: {}, {}",
                            v, err
                        ))
                    })?);
                }
                "client_port" => {
                    result.client_port = Some(parse_number_range::<u16>(v).map_err(|err| {
                        RtspMessageError::InvalidRtspMessageFormat(format!(
                            "[transport header] parse port range for client_port failed: {}, {}",
                            v, err
                        ))
                    })?)
                }
                "server_port" => {
                    result.server_port = Some(parse_number_range::<u16>(v).map_err(|err| {
                        RtspMessageError::InvalidRtspMessageFormat(format!(
                            "[transport header] parse port range for server_port failed: {}, {}",
                            v, err
                        ))
                    })?)
                }
                _ => {
                    // ignore
                }
            }
        }

        Ok(result)
    }
}
