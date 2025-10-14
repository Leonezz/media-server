mod channel_number;
pub use channel_number::*;
mod lifetime;
pub use lifetime::*;
mod xor_peer_address;
pub use xor_peer_address::*;
mod data;
pub use data::*;
mod xor_relayed_address;
pub use xor_relayed_address::*;
mod requested_address_family;
pub use requested_address_family::*;
mod even_port;
pub use even_port::*;
mod requested_transport;
pub use requested_transport::*;
mod dont_fragment;
pub use dont_fragment::*;
mod reservation_token;
pub use reservation_token::*;
mod additional_address_family;
pub use additional_address_family::*;
mod address_error_code;
pub use address_error_code::*;
mod icmp;
pub use icmp::*;

#[cfg(test)]
mod test {
    use stun_formats::attributes::{AttributeExtEntry, AttributeExtStatic};

    use crate::attributes::rfc8656::*;

    fn registered(attr_type: u16) -> bool {
        inventory::iter::<AttributeExtEntry>
            .into_iter()
            .find(|entry| entry.attr_type == attr_type)
            .is_some()
    }
    #[test]
    fn all_registered() {
        let types = [
            AdditionalAddressFamilyAttribute::STATIC_ATTR_TYPE,
            AddressErrorCodeAttrbute::STATIC_ATTR_TYPE,
            ChannelNumberAttribute::STATIC_ATTR_TYPE,
            DataAttribute::STATIC_ATTR_TYPE,
            DontFragmentAttribute::STATIC_ATTR_TYPE,
            EvenPortAttribute::STATIC_ATTR_TYPE,
            IcmpAttribute::STATIC_ATTR_TYPE,
            LifeTimeAttribute::STATIC_ATTR_TYPE,
            RequestedAddressFamilyAttribute::STATIC_ATTR_TYPE,
            RequestedTransportAttribute::STATIC_ATTR_TYPE,
            ReservationTokenAttribute::STATIC_ATTR_TYPE,
            XorPeerAddressAttribute::STATIC_ATTR_TYPE,
            XorRelayedAddressAttribute::STATIC_ATTR_TYPE,
        ];

        for t in types {
            assert!(registered(t))
        }
    }

    #[test]
    fn unregistered() {
        assert!(!registered(0x00));
    }
}
