use stun_formats::{
    define_error_code,
    error_codes::{ErrorCodeExtDynamic, ErrorCodeExtStatic},
};

// 403 (Forbidden):
// The request was valid but cannot be performed due to administrative or similar restrictions.
define_error_code!(403, FORBIDDEN, "Forbidden");

// 437 (Allocation Mismatch):
// A request was received by the server that requires an allocation to be in place,
// but no allocation exists, or a request was received that requires no allocation, but an allocation exists.
define_error_code!(437, ALLOCATION_MISMATCH, "Allocation Mismatch");

// 440 (Address Family not Supported):
// The server does not support the address family requested by the client.
define_error_code!(
    440,
    ADDRESS_FAMILY_NOT_SUPPORTED,
    "Address Family not Supported"
);

// 441 (Wrong Credentials):
// (Wrong Credentials): The credentials in the (non-Allocate) request do not match those used to create the allocation.
define_error_code!(441, WRONG_CREDENTIALS, "Wrong Credentials");

// 442 (Unsupported Transport Protocol):
// The Allocate request asked the server to use a transport protocol between the server and the peer that the server does not support.
// NOTE: This does NOT refer to the transport protocol used in the 5-tuple.
define_error_code!(
    442,
    UNSUPPORTED_TRANSPORT_PROTOCOL,
    "Unsupported Transport Protocol"
);

// 443 (Peer Address Family Mismatch):
// A peer address is part of a different address family than that of the relayed transport address of the allocation.
define_error_code!(
    443,
    PEER_ADDRESS_FAMILY_MISMATCH,
    "Peer Address Family Mismatch"
);

// 486 (Allocation Quota Reached):
// No more allocations using this username can be created at the present time.
define_error_code!(486, ALLOCATION_QUOTA_REACHED, "Allocation Quota Reached");

// 508 (Insufficient Capacity):
// The server is unable to carry out the request due to some capacity limit being reached.
// In an Allocate response, this could be due to the server having no more relayed transport addresses available at that time,
// having none with the requested properties, or the one that corresponds to the specified reservation token is not available.
define_error_code!(508, INSUFFICIENT_CAPACITY, "Insufficient Capacity");

#[cfg(test)]
mod test {
    use stun_formats::error_codes::{ErrorCodeExtStatic, from_code, from_code_with_reason};

    use crate::error_codes::rfc8656::{
        ADDRESS_FAMILY_NOT_SUPPORTED, ALLOCATION_MISMATCH, ALLOCATION_QUOTA_REACHED, FORBIDDEN,
        INSUFFICIENT_CAPACITY, PEER_ADDRESS_FAMILY_MISMATCH, UNSUPPORTED_TRANSPORT_PROTOCOL,
        WRONG_CREDENTIALS,
    };
    #[test]
    fn print_all_registered() {
        let all_codes = [
            FORBIDDEN::STATIC_CODE,
            ALLOCATION_MISMATCH::STATIC_CODE,
            ADDRESS_FAMILY_NOT_SUPPORTED::STATIC_CODE,
            WRONG_CREDENTIALS::STATIC_CODE,
            UNSUPPORTED_TRANSPORT_PROTOCOL::STATIC_CODE,
            PEER_ADDRESS_FAMILY_MISMATCH::STATIC_CODE,
            ALLOCATION_QUOTA_REACHED::STATIC_CODE,
            INSUFFICIENT_CAPACITY::STATIC_CODE,
        ];
        for code in all_codes {
            let err = from_code(code);
            assert!(err.is_some());
            let err = err.unwrap();
            println!("{:?}", err);
            assert_eq!(
                format!("{:?}", err),
                format!("{}({}): {}", err.name(), err.code(), err.reason())
            );
        }
    }

    #[test]
    fn test_with_reason() {
        let code = 400;
        let err = from_code_with_reason(code, "BAD BAD REQUEST");
        assert!(err.is_some());
        let err = err.unwrap();
        assert_eq!(err.reason(), "BAD BAD REQUEST");
        assert_eq!(format!("{:?}", err), "BAD_REQUEST(400): BAD BAD REQUEST");
    }

    #[test]
    fn test_unregistered() {
        let code = 402;
        assert!(from_code(code).is_none());
    }
}
