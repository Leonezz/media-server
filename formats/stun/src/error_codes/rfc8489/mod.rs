use super::{ErrorCodeExtDynamic, ErrorCodeExtStatic};
use crate::define_error_code;

// 300 Try Alternate:
// The client should contact an alternate server for this request.
// This error response MUST only be sent if the request included either
// a USERNAME or USERHASH attribute and a valid MESSAGE-INTEGRITY or
// MESSAGE-INTEGRITY-SHA256 attribute; otherwise,
// it MUST NOT be sent and error code 400 (Bad Request) is suggested.
// This error response MUST be protected with the MESSAGE-INTEGRITY or
// MESSAGE-INTEGRITY-SHA256 attribute, and receivers MUST validate the MESSAGE-INTEGRITY or
// MESSAGEINTEGRITY-SHA256 of this response before redirecting themselves to an alternate server.
// Note: Failure to generate and validate message integrity for a 300 response allows an
// on-path attacker to falsify a 300 response thus causing subsequent STUN messages to be sent to a victim.
define_error_code!(300, TRY_ALTERNATE, "Try Alternate");

// 400 Bad Request:
// The request was malformed.
// The client SHOULD NOT retry the request without modification from the previous attempt.
// The server may not be able to generate a valid MESSAGE-INTEGRITY or MESSAGE-INTEGRITY-SHA256 for this error,
// so the client MUST NOT expect a valid MESSAGE-INTEGRITY or MESSAGEINTEGRITY-SHA256 attribute on this response.
define_error_code!(400, BAD_REQUEST, "Bad Request");

// 401 Unauthenticated:
// The request did not contain the correct credentials to proceed.
// The client should retry the request with proper credentials.
define_error_code!(401, UNAUTHENTICATED, "Unauthenticated");

// 420 Unknown Attribute:
// The server received a STUN packet containing a comprehension-required attribute that it did not understand.
// The server MUST put this unknown attribute in the UNKNOWNATTRIBUTE attribute of its error response.
define_error_code!(420, UNKNOWN_ATTRIBUTE, "Unknown Attribute");

// 438 Stale Nonce:
// The NONCE used by the client was no longer valid.
// The client should retry, using the NONCE provided in the response.
define_error_code!(438, NONCE, "Stale Nonce");

// 500 Server Error:
// The server has suffered a temporary error. The client should try again.
define_error_code!(500, SERVER_ERROR, "Server Error");

#[cfg(test)]
mod test {
    use crate::error_codes::{from_code, from_code_with_reason};
    #[test]
    fn print_all_registered() {
        let all_codes = [300, 400, 401, 420, 438, 500];
        for code in all_codes {
            let err = from_code(code);
            assert!(err.is_some());
            println!("{:?}", err.unwrap());
        }
    }

    #[test]
    fn test_with_reason() {
        let code = 400;
        let err = from_code_with_reason(code, "BAD BAD REQUEST");
        assert!(err.is_some());
        assert_eq!(err.unwrap().reason(), "BAD BAD REQUEST");
    }

    #[test]
    fn test_unregistered() {
        let code = 402;
        assert!(from_code(code).is_none());
    }
}
