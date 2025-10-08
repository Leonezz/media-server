use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Protocol {
    Tcp,
    Udp,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str((*self).into())
    }
}

impl FromStr for Protocol {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tcp" => Ok(Self::Tcp),
            "udp" => Ok(Self::Udp),
            _ => Err(s.to_owned()),
        }
    }
}

impl From<Protocol> for &'static str {
    fn from(value: Protocol) -> Self {
        match value {
            Protocol::Tcp => "tcp",
            Protocol::Udp => "udp",
        }
    }
}
