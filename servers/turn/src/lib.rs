use std::net::IpAddr;

use iana_formats::addrress_family::AddressFamilyStatic;

pub mod allocation;
pub mod authenticate_info;
pub mod channel_binding;
pub mod errors;
pub mod five_tuple;
pub mod permission;
pub mod server;
pub mod session;

pub fn get_address_family<Addr: Into<IpAddr>>(
    addr: Addr,
) -> iana_formats::addrress_family::AddressFamily {
    let ip: IpAddr = addr.into();
    if ip.is_ipv4() {
        return iana_formats::addrress_family::IPv4::ADDRESS_FAMILY;
    }
    if ip.is_ipv6() {
        return iana_formats::addrress_family::IPv6::ADDRESS_FAMILY;
    }
    unreachable!("unknown ip family")
}
