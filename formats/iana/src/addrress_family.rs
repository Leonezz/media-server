use std::{
    fmt,
    net::{IpAddr, SocketAddr},
};

use crate::rfc_url;

// https://www.iana.org/assignments/address-family-numbers/address-family-numbers.xhtml
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct AddressFamily(#[allow(unused)] u16);

impl AddressFamily {
    pub fn new(value: u16) -> Option<Self> {
        if is_registered(value) {
            return Some(Self(value));
        }
        None
    }

    pub fn inner(&self) -> u16 {
        self.0
    }
}

impl fmt::Display for AddressFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let a = from_address_family(*self);
        if let Some(a) = a {
            write!(f, "{}({})", a.name(), a.decimal())
        } else {
            write!(f, "unknown({})", self.0)
        }
    }
}

impl fmt::Debug for AddressFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let a = from_address_family(*self);
        if let Some(a) = a {
            f.debug_struct("AddressFamily")
                .field("decimal", &a.decimal())
                .field("name", &a.name())
                .field("description", &a.description())
                .field("reference", &a.reference())
                .finish()
        } else {
            write!(f, "unknown({})", self.0)
        }
    }
}

pub trait AddressFamilyStatic {
    const DECIMAL: u16;
    const ADDRESS_FAMILY: AddressFamily;
    const NAME: &'static str;
    const DESCRIPTION: &'static str;
    const REFERENCE: &'static str;
}

pub trait AddressFamilyDynamic: Send + Sync {
    fn decimal(&self) -> u16;
    fn address_family(&self) -> AddressFamily;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn reference(&self) -> &'static str;
}

struct AddressFamilyEntry {
    number: u16,
    factory: fn() -> Box<dyn AddressFamilyDynamic>,
}

inventory::collect!(AddressFamilyEntry);

fn entry(number: u16) -> Option<&'static AddressFamilyEntry> {
    inventory::iter::<AddressFamilyEntry>
        .into_iter()
        .find(|item| item.number == number)
}

pub fn is_registered(number: u16) -> bool {
    entry(number).is_some()
}

pub fn from_number(number: u16) -> Option<Box<dyn AddressFamilyDynamic>> {
    entry(number).map(|item| (item.factory)())
}

pub fn from_address_family(address_family: AddressFamily) -> Option<Box<dyn AddressFamilyDynamic>> {
    from_number(address_family.0)
}

pub fn for_address(addr: &SocketAddr) -> AddressFamily {
    for_ip_address(&addr.ip())
}

pub fn for_ip_address(ip_addr: &IpAddr) -> AddressFamily {
    match ip_addr {
        IpAddr::V4(_) => IPv4::ADDRESS_FAMILY,
        IpAddr::V6(_) => IPv6::ADDRESS_FAMILY,
    }
}

macro_rules! define_address_family {
    ($number: tt, $name: tt, $description: expr, $reference: expr) => {
        #[allow(non_camel_case_types)]
        pub struct $name;
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", $name, $number)
            }
        }
        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct(stringify!($name))
                    .field("decimal", &$number)
                    .field("description", &$description)
                    .field("reference", &$reference)
                    .finish()
            }
        }
        impl AddressFamilyStatic for $name {
            const DECIMAL: u16 = $number;
            const ADDRESS_FAMILY: AddressFamily = AddressFamily($number);
            const NAME: &'static str = stringify!($name);
            const DESCRIPTION: &'static str = $description;
            const REFERENCE: &'static str = $reference;
        }
        impl AddressFamilyDynamic for $name {
            fn decimal(&self) -> u16 {
                Self::DECIMAL
            }
            fn address_family(&self) -> AddressFamily {
                Self::ADDRESS_FAMILY
            }
            fn name(&self) -> &'static str {
                Self::NAME
            }
            fn description(&self) -> &'static str {
                Self::DESCRIPTION
            }
            fn reference(&self) -> &'static str {
                Self::REFERENCE
            }
        }
        inventory::submit! {
            AddressFamilyEntry {
                number: $number,
                factory: || Box::new($name{})
            }
        }
    };
}

define_address_family!(1, IPv4, "IP (IP version 4)", "");
define_address_family!(2, IPv6, "IP6 (IP version 6)", "");
define_address_family!(3, NSAP, "NSAP", "");
define_address_family!(4, HDLC, "HDLC (8-bit multidrop)", "");
define_address_family!(5, BBN_1822, "BBN 1822", "");
define_address_family!(
    6,
    A_802,
    "802 (includes all 802 media plus Ethernet \"canonical format\")",
    ""
);
define_address_family!(7, E_163, "E.163", "");
define_address_family!(8, E_164, "E.164 (SMDS, Frame Relay, ATM)", "");
define_address_family!(9, F_69, "F.69 (Telex)", "");
define_address_family!(10, X_121, "X.121 (X.25, Frame Relay)", "");
define_address_family!(11, IPX, "IPX", "");
define_address_family!(12, Appletalk, "Appletalk", "");
define_address_family!(13, DecnetIV, "Decnet IV", "");
define_address_family!(14, BanyanVines, "Banyan Vines", "");
define_address_family!(
    15,
    E_164_NSAP,
    "E.164 with NSAP format subaddress",
    "[ATM Forum UNI 3.1. October 1995.][Andy Malis](mailto:agmalis&gmail.com)"
);
define_address_family!(16, DNS, "DNS (Domain Name System)", "");
define_address_family!(
    17,
    DistinguishedName,
    "Distinguished Name",
    "[Charles Lynn](mailto:clynn&bbn.com)"
);
define_address_family!(
    18,
    ASNumber,
    "AS Number",
    "[Charles Lynn](mailto:clynn&bbn.com)"
);
define_address_family!(
    19,
    XTP_IPv4,
    "XTP over IP version 4",
    "[Mike Saul](mailto:mike&nentat.com)"
);
define_address_family!(
    20,
    XTP_IPv6,
    "XTP over IP version 6",
    "[Mike Saul](mailto:mike&nentat.com)"
);
define_address_family!(
    21,
    XTP_NATIVE,
    "XTP native mode XTP",
    "[Mike Saul](mailto:mike&nentat.com)"
);
define_address_family!(
    22,
    FCWWPN,
    "Fibre Channel World-Wide Port Name",
    "[Mark Bakke](mailto:mbakke&cisco.com)"
);
define_address_family!(
    23,
    FCWWNN,
    "Fibre Channel World-Wide Node Name",
    "[Mark Bakke](mailto:mbakke&cisco.com)"
);
define_address_family!(24, GWID, "GWID", "[Subra Hegde](mailto:subrah&cisco.com)");
define_address_family!(
    25,
    AFI,
    "AFI for L2VPN information",
    rfc_url!(rfc4761, rfc074)
);
define_address_family!(
    26,
    MPLS_TP_EI,
    "MPLS-TP Section Endpoint Identifier",
    rfc_url!(rfc7212)
);
define_address_family!(
    27,
    MPLS_TP_LSP_EI,
    "MPLS-TP LSP Endpoint Identifier",
    rfc_url!(rfc7212)
);
define_address_family!(
    28,
    MPLS_TP_PEI,
    "MPLS-TP Pseudowire Endpoint Identifier",
    rfc_url!(rfc7212)
);
define_address_family!(
    29,
    MP_IPv4,
    "MT IP: Multi-Topology IP version 4",
    rfc_url!(rfc7307)
);
define_address_family!(
    30,
    MT_IPv6,
    "MT IPv6: Multi-Topology IP version 6",
    rfc_url!(rfc7307)
);
define_address_family!(31, BGP_SFC, "BGP SFC", rfc_url!(rfc9015));
define_address_family!(
    16384,
    EIGRP_COMMON,
    "EIGRP Common Service Family",
    "[Donnie Savage](mailto:dsavage&cisco.com)"
);
define_address_family!(
    16385,
    EIGRP_IPv4,
    "EIGRP IPv4 Service Family",
    "[Donnie Savage](mailto:dsavage&cisco.com)"
);
define_address_family!(
    16386,
    EIGRP_IPv6,
    "EIGRP IPv6 Service Family",
    "[Donnie Savage](mailto:dsavage&cisco.com)"
);
define_address_family!(
    16387,
    LCAF,
    "LISP Canonical Address Format (LCAF)",
    "[David Meyer](mailto:dmm&1-4-5.net)"
);
define_address_family!(16388, BGP_LS, "BGP-LS", rfc_url!(rfc9552));
define_address_family!(16389, MAC_48, "48-bit MAC", rfc_url!(rfc7042));
define_address_family!(16390, MAC_64, "64-bit MAC", rfc_url!(rfc7042));
define_address_family!(16391, OUI, "OUI", rfc_url!(rfc7961));
define_address_family!(16392, MAC_24, "MAC/24", rfc_url!(rfc7961));
define_address_family!(16393, MAC_40, "MAC/40", rfc_url!(rfc7961));
define_address_family!(16394, IPv6_64, "IPv6/64", rfc_url!(rfc7961));
define_address_family!(16395, RBPI, "RBridge Port ID", rfc_url!(rfc7961));
define_address_family!(16396, TRILL, "TRILL Nickname", rfc_url!(rfc7455));
define_address_family!(
    16397,
    UUID,
    "Universally Unique Identifier (UUID)",
    "[Nischal Sheth](mailto:nischal.sheth&gmail.com)"
);
define_address_family!(
    16398,
    RPAFI,
    "Routing Policy AFI",
    "[draft-ietf-idr-rpd-02](https://datatracker.ietf.org/doc/draft-ietf-idr-rpd/02/)"
);
define_address_family!(
    16399,
    MPLS,
    "MPLS Namespaces",
    "[draft-kaliraj-bess-bgp-sig-private-mpls-labels-03](https://datatracker.ietf.org/doc/draft-kaliraj-bess-bgp-sig-private-mpls-labels/03/)"
);
