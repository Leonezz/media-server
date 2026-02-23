mod mapped_address;
pub use mapped_address::{ADDRESS_FAMILY_V4, ADDRESS_FAMILY_V6, MappedAddressAttribute};
mod xor_mapped_address;
pub use xor_mapped_address::XorMappedAddressAttribute;
mod username;
pub use username::UserNameAttribute;
mod userhash;
pub use userhash::UserHashAttribute;
mod message_integrity;
pub use message_integrity::MessageIntegrityAttribute;
mod message_integrity_sha256;
pub use message_integrity_sha256::MessageIntegritySHA256Attribute;
mod fingerprint;
pub use fingerprint::FingerPrintAttribute;

pub mod error_code;
pub use error_code::ErrorCodeAttribute;

mod realm;
pub use realm::RealmAttribute;
mod nonce;
pub use nonce::NonceAttribute;
mod password_algorithms;
pub use password_algorithms::PasswordAlgorithmsAttribute;
mod password_algorithm;
pub use password_algorithm::PasswordAlgorithmAttribute;
mod unknown_attributes;
pub use unknown_attributes::UnknownAttributesAttribute;
mod software;
pub use software::SoftwareAttribute;
mod alternate_server;
pub use alternate_server::AlternateServerAttribute;
mod alternate_domain;
pub use alternate_domain::AlternateDomainAttribute;

#[cfg(test)]
mod test {
    use crate::attributes::{AttributeExtEntry, AttributeExtStatic, rfc8489::*};

    fn registered(attr_type: u16) -> bool {
        inventory::iter::<AttributeExtEntry>
            .into_iter()
            .find(|entry| entry.attr_type == attr_type)
            .is_some()
    }
    #[test]
    fn all_registered() {
        let types = [
            MappedAddressAttribute::STATIC_ATTR_TYPE,
            XorMappedAddressAttribute::STATIC_ATTR_TYPE,
            UserNameAttribute::STATIC_ATTR_TYPE,
            UserHashAttribute::STATIC_ATTR_TYPE,
            MessageIntegrityAttribute::STATIC_ATTR_TYPE,
            MessageIntegritySHA256Attribute::STATIC_ATTR_TYPE,
            FingerPrintAttribute::STATIC_ATTR_TYPE,
            ErrorCodeAttribute::STATIC_ATTR_TYPE,
            RealmAttribute::STATIC_ATTR_TYPE,
            NonceAttribute::STATIC_ATTR_TYPE,
            PasswordAlgorithmsAttribute::STATIC_ATTR_TYPE,
            PasswordAlgorithmAttribute::STATIC_ATTR_TYPE,
            UnknownAttributesAttribute::STATIC_ATTR_TYPE,
            SoftwareAttribute::STATIC_ATTR_TYPE,
            AlternateServerAttribute::STATIC_ATTR_TYPE,
            AlternateDomainAttribute::STATIC_ATTR_TYPE,
        ];

        for t in types {
            assert!(registered(t))
        }
    }

    #[test]
    fn unregistered() {
        assert!(!registered(0x0018));
    }
}
