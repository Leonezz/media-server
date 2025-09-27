#![feature(ip_as_octets)]
#![feature(buf_read_has_data_left)]
//! see: RFC 8489 Session Traversal Utilities for NAT (STUN)
pub mod attribute;
mod attributes;
pub mod errors;
pub mod header;
pub use attributes::rfc8489;
pub mod builder;
pub mod message;
pub mod methods;

pub const STUN_URI_SCHEMA: &str = "stun";
pub const STUN_URI_SCHEMA_SECURE: &str = "stuns";
