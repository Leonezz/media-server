use std::{fmt, str::FromStr};

use num::Integer;

use crate::errors::RtspMessageError;

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

impl fmt::Debug for TransportProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RtpAvpUdp => f.write_str("RTP/AVP/UDP"),
            Self::RtpAvpTcp => f.write_str("RTP/AVP/TCP"),
            Self::RtpAvpfUdp => f.write_str("RTP/AVPF/UDP"),
            Self::RtpAvpfTcp => f.write_str("RTP/AVPF/TCP"),
            Self::RtpSavpUdp => f.write_str("RTP/SAVP/UDP"),
            Self::RtpSavpTcp => f.write_str("RTP/SAVP/TCP"),
            Self::RtpSavpfUdp => f.write_str("RTP/SAVPF/UDP"),
            Self::RtpSavpfTcp => f.write_str("RTP/SAVPF/TCP"),
        }
    }
}

pub enum TransportMode {
    Play,
    Record,
    Other(String),
}

impl fmt::Debug for TransportMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Play => f.write_str("PLAY"),
            Self::Record => f.write_str("RECORD"),
            Self::Other(str) => f.write_str(str),
        }
    }
}

pub enum TransportCast {
    Unicast,
    Multicast,
}

impl fmt::Debug for TransportCast {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unicast => f.write_str("unicast"),
            Self::Multicast => f.write_str("multicast"),
        }
    }
}

pub enum Addr {
    Port(u16),
    Host(String),
    HostPort((String, u16)),
}

impl fmt::Debug for Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Port(port) => f.write_fmt(format_args!(":{}", port)),
            Self::Host(host) => f.write_str(host),
            Self::HostPort((host, port)) => f.write_fmt(format_args!("{}:{}", host, port)),
        }
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

impl fmt::Debug for Setup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => f.write_str("active"),
            Self::Passive => f.write_str("passive"),
            Self::Actpass => f.write_str("actpass"),
        }
    }
}

#[derive(Clone, Copy)]
pub enum Connection {
    New,
    Existing,
}

impl fmt::Debug for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::New => f.write_str("new"),
            Self::Existing => f.write_str("existing"),
        }
    }
}

#[derive(Default)]
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

impl fmt::Debug for TransportHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(profile) = &self.profile {
            writeln!(f, "profile: {:?}", profile)?;
        }

        if let Some(cast) = &self.cast {
            writeln!(f, "cast: {:?}", cast)?;
        }

        if let Some((a, b)) = self.interleaved {
            writeln!(f, "interleaved: {}-{}", a, b)?;
        }

        if let Some(ttl) = self.ttl {
            writeln!(f, "ttl: {}", ttl)?;
        }

        if let Some(layers) = self.layers {
            writeln!(f, "layers: {}", layers)?;
        }

        if !self.ssrc_list.is_empty() {
            writeln!(
                f,
                "ssrc: {}",
                self.ssrc_list
                    .iter()
                    .map(|ssrc| ssrc.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )?;
        }

        if !self.mode.is_empty() {
            writeln!(
                f,
                "mode: {}",
                self.mode
                    .iter()
                    .map(|mode| format!("{:?}", mode))
                    .collect::<Vec<_>>()
                    .join(", ")
            )?;
        }

        if !self.dest_addr.is_empty() {
            writeln!(
                f,
                "dest_addr: {}",
                self.dest_addr
                    .iter()
                    .map(|addr| format!("{:?}", addr))
                    .collect::<Vec<_>>()
                    .join(", ")
            )?;
        }

        if !self.src_addr.is_empty() {
            writeln!(
                f,
                "src_addr: {}",
                self.src_addr
                    .iter()
                    .map(|addr| format!("{:?}", addr))
                    .collect::<Vec<_>>()
                    .join(", ")
            )?;
        }

        if let Some(setup) = self.setup {
            writeln!(f, "setup: {:?}", setup)?;
        }

        if let Some(conn) = self.connection {
            writeln!(f, "connection: {:?}", conn)?;
        }

        writeln!(f, "RTCP-mux: {}", self.rtcp_mux)?;
        if let Some(key) = &self.mikey {
            writeln!(f, "MIKEY: {}", key)?;
        }

        writeln!(f, "append: {}", self.append)?;
        if let Some(dest) = &self.destination {
            writeln!(f, "destination: {}", dest)?;
        }

        if let Some((a, b)) = self.port {
            writeln!(f, "port: {}-{}", a, b)?;
        }

        if let Some((a, b)) = self.client_port {
            writeln!(f, "client_port: {}-{}", a, b)?;
        }

        if let Some((a, b)) = self.server_port {
            writeln!(f, "server_port: {}-{}", a, b)?;
        }

        Ok(())
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
