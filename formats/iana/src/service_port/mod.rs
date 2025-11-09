mod define;
pub use define::*;
// https://www.iana.org/assignments/service-names-port-numbers/service-names-port-numbers.xhtml

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegisteredPort {
    Port(u16),
    PortRange(u16, u16),
}

impl std::fmt::Display for RegisteredPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Port(p) => write!(f, "{}", p),
            Self::PortRange(min, max) => write!(f, "{}-{}", min, max),
        }
    }
}

impl RegisteredPort {
    pub fn new(port: u16) -> Self {
        Self::Port(port)
    }
    pub fn new_range(min: u16, max: u16) -> Self {
        Self::PortRange(min, max)
    }
    pub const fn min_port(&self) -> u16 {
        match self {
            Self::Port(p) => *p,
            Self::PortRange(p, _) => *p,
        }
    }
    pub const fn max_port(&self) -> u16 {
        match self {
            Self::Port(p) => *p,
            Self::PortRange(_, p) => *p,
        }
    }
    pub fn in_range(&self, port: u16) -> bool {
        match self {
            Self::Port(p) => p.eq(&port),
            Self::PortRange(min, max) => port >= *min && port <= *max,
        }
    }
}

pub trait ServicePortStatic {
    const SERVICE_NAME: &'static str;
    const PORT: RegisteredPort;
    const DESCRIPTION: &'static str;
}

pub trait ServicePortDynamic {
    fn service_name(&self) -> &'static str;
    fn port(&self) -> RegisteredPort;
    fn description(&self) -> &'static str;
}

struct ServicePortEntry {
    service_name: &'static str,
    factory: fn() -> Box<dyn ServicePortDynamic>,
}

inventory::collect!(ServicePortEntry);

fn entry(service_name: &str) -> Option<&'static ServicePortEntry> {
    inventory::iter::<ServicePortEntry>
        .into_iter()
        .find(|item| item.service_name == service_name)
}

pub fn from_name(name: &str) -> Option<Box<dyn ServicePortDynamic>> {
    entry(name).map(|item| (item.factory)())
}

#[macro_export]
macro_rules! define_service_port {
    ($id: ident, $name: tt, $port: tt, $description: tt) => {
        #[allow(non_camel_case_types)]
        pub struct $id;
        impl std::fmt::Display for $id {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(
                    f,
                    "{}({}) port: {}",
                    Self::SERVICE_NAME,
                    stringify!($id),
                    Self::PORT
                )
            }
        }
        impl ServicePortStatic for $id {
            const SERVICE_NAME: &'static str = $name;
            const PORT: RegisteredPort = RegisteredPort::Port($port);
            const DESCRIPTION: &'static str = $description;
        }

        impl ServicePortDynamic for $id {
            fn service_name(&self) -> &'static str {
                Self::SERVICE_NAME
            }
            fn port(&self) -> RegisteredPort {
                Self::PORT
            }
            fn description(&self) -> &'static str {
                Self::DESCRIPTION
            }
        }

        inventory::submit! {
            ServicePortEntry {
                service_name: $name,
                factory: || Box::new($id{})
            }
        }
    };
    ($id: ident, $name: tt, $port_min: tt, $port_max: tt, $description: tt) => {
        #[allow(non_camel_case_types)]
        pub struct $id;
        impl std::fmt::Display for $id {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(
                    f,
                    "{}({}) port: {}",
                    Self::SERVICE_NAME,
                    stringify!($id),
                    Self::PORT
                )
            }
        }
        impl ServicePortStatic for $id {
            const SERVICE_NAME: &'static str = $name;
            const PORT: RegisteredPort = RegisteredPort::PortRange($port_min, $port_max);
            const DESCRIPTION: &'static str = $description;
        }

        impl ServicePortDynamic for $id {
            fn service_name(&self) -> &'static str {
                Self::SERVICE_NAME
            }
            fn port(&self) -> RegisteredPort {
                Self::PORT
            }
            fn description(&self) -> &'static str {
                Self::DESCRIPTION
            }
        }

        inventory::submit! {
            ServicePortEntry {
                service_name: $name,
                factory: || Box::new($id{})
            }
        }
    };
}
